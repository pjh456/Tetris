#!/usr/bin/env python3
"""Afterstate value-DQN trainer for Tetris (08-10).

Replaces the PPO-over-actions approach (which plateaued without actively
clearing) with the prior-art-proven afterstate recipe (nuno-faria): a small MLP
scores the 61-dim afterstate of every legal placement, the agent picks the
max-value placement (epsilon-greedy), and the value net is trained with
afterstate Q-learning `V(s_a) <- r + gamma * max_a' V(s'_a')`.

The value net is a small MLP (61 -> 64 -> 64 -> 1) so its weights export to the
same weights.json schema (output_dim=1) that tetris-infer's MLP forward reads
(08-11). Uses the Rust env directly (TetrisEnv from tetris_ai) — no SB3/gym, the
afterstate loop needs a variable-size action set per step.
"""

from __future__ import annotations

import argparse
import functools
import os
import random
from collections import deque
from dataclasses import dataclass

import gymnasium as gym
import numpy as np
import torch
import torch.nn as nn
from gymnasium.vector import AsyncVectorEnv, SyncVectorEnv

import tetris_env  # noqa: F401  registers the "Tetris-v0" gym env

OBS_DIM = 71
TAU = 0.005  # Polyak soft update rate for target network


def set_seed(seed: int) -> None:
    """Full reproducibility (pytorch-patterns)."""
    torch.manual_seed(seed)
    torch.cuda.manual_seed_all(seed)
    np.random.seed(seed)
    random.seed(seed)


class ValueMLP(nn.Module):
    """Scores a board afterstate. Input: (batch, OBS_DIM). Output: (batch, 1)."""

    def __init__(self, input_dim: int = OBS_DIM, hidden: int = 64) -> None:
        super().__init__()
        self.ln1 = nn.LayerNorm(input_dim, eps=1e-5)
        self.fc1 = nn.Linear(input_dim, hidden)
        self.ln2 = nn.LayerNorm(hidden, eps=1e-5)
        self.fc2 = nn.Linear(hidden, hidden)
        self.fc3 = nn.Linear(hidden, 1)
        self._init_weights()

    def _init_weights(self) -> None:
        gain = nn.init.calculate_gain("tanh")
        for name, mod in self.named_modules():
            if isinstance(mod, nn.Linear) and "fc" in name:
                nn.init.xavier_uniform_(mod.weight, gain=gain)
                if mod.bias is not None:
                    nn.init.zeros_(mod.bias)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        x = self.ln1(x)
        x = torch.tanh(self.fc1(x))
        x = self.ln2(x)
        x = torch.tanh(self.fc2(x))
        return self.fc3(x)


@dataclass
class Transition:
    chosen: np.ndarray  # (61,) afterstate features of the played placement
    reward: float
    next_feats: np.ndarray  # (m, 61) afterstates available at s'; (0, 61) if done
    done: bool


class ReplayBuffer:
    def __init__(self, capacity: int) -> None:
        self.buf: deque[Transition] = deque(maxlen=capacity)

    def push(self, t: Transition) -> None:
        self.buf.append(t)

    def sample(self, batch_size: int) -> list[Transition]:
        return random.sample(self.buf, batch_size)

    def __len__(self) -> int:
        return len(self.buf)


def epsilon_at(step: int, total: int, end_frac: float) -> float:
    """Linear anneal 1.0 -> 0.0 over the first `end_frac` of training."""
    cutoff = max(1, int(total * end_frac))
    return max(0.0, 1.0 - step / cutoff)


@torch.no_grad()
def pick_action(
    net: ValueMLP,
    actions: np.ndarray,
    feats: np.ndarray,
    epsilon: float,
    device: torch.device,
) -> tuple[int, np.ndarray]:
    """Epsilon-greedy over afterstate values. Returns (env_action, chosen_feats)."""
    n = len(actions)
    if n == 0:
        return 0, np.zeros(OBS_DIM, dtype=np.float32)
    if random.random() < epsilon:
        idx = random.randrange(n)
    else:
        net.eval()
        x = torch.from_numpy(feats).float().to(device)  # (n, 61)
        values = net(x).squeeze(1)  # (n,)
        idx = int(torch.argmax(values).item())
    return int(actions[idx]), feats[idx].copy()


def train_step(
    online: ValueMLP,
    target: ValueMLP,
    batch: list[Transition],
    optimizer: torch.optim.Optimizer,
    gamma: float,
    device: torch.device,
) -> float:
    online.train()
    chosen = torch.from_numpy(np.stack([t.chosen for t in batch])).float().to(device)  # (B,61)
    rewards = torch.tensor([t.reward for t in batch], dtype=torch.float32, device=device)  # (B,)

    # max_a' V(s'_a') per sample, batched into ONE forward over all next afterstates.
    next_max = torch.zeros(len(batch), dtype=torch.float32, device=device)
    flat: list[np.ndarray] = []
    spans: list[tuple[int, int, int]] = []  # (sample_idx, start, end)
    cursor = 0
    for i, t in enumerate(batch):
        if t.done or t.next_feats.shape[0] == 0:
            continue
        m = t.next_feats.shape[0]
        flat.append(t.next_feats)
        spans.append((i, cursor, cursor + m))
        cursor += m
    if flat:
        with torch.no_grad():
            target.eval()
            allx = torch.from_numpy(np.concatenate(flat, axis=0)).float().to(device)  # (sumM,61)
            allv = target(allx).squeeze(1)  # (sumM,)
        for i, s, e in spans:
            next_max[i] = allv[s:e].max()

    targets = rewards + gamma * next_max  # done samples keep next_max == 0 -> targets == reward
    preds = online(chosen).squeeze(1)  # (B,)
    loss = nn.functional.mse_loss(preds, targets)

    optimizer.zero_grad(set_to_none=True)
    loss.backward()
    torch.nn.utils.clip_grad_norm_(online.parameters(), max_norm=1.0)
    optimizer.step()
    return float(loss.item())


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser()
    p.add_argument("--episodes", type=int, default=2000)
    p.add_argument("--max-steps", type=int, default=10_000)
    p.add_argument("--replay-size", type=int, default=20_000)
    p.add_argument("--batch-size", type=int, default=512)
    p.add_argument("--gamma", type=float, default=0.95)
    p.add_argument("--lr", type=float, default=1e-3)
    p.add_argument("--epsilon-end-frac", type=float, default=0.75)
    p.add_argument("--epsilon-min", type=float, default=0.02)
    # Anneal epsilon over a FIXED number of episodes (decoupled from --episodes),
    # so raising --episodes lengthens the low-epsilon LEARNING tail instead of
    # stretching the random-exploration phase.
    p.add_argument("--epsilon-decay-episodes", type=int, default=1500)
    p.add_argument("--target-sync", type=int, default=500)
    p.add_argument("--train-start", type=int, default=1000)
    # Speed: train once every N env steps (not every step). nuno-faria trains ~1x
    # per episode; every-step is ~19x more backprops than needed.
    p.add_argument("--train-interval", type=int, default=4)
    # Tiny MLP (61->64->64->1): GPU host<->device transfer often costs more than
    # the matmul. "auto" picks cuda if present; try "cpu" for this small net.
    p.add_argument("--device", type=str, default="auto", choices=["auto", "cpu", "cuda"])
    # Parallel environments: AsyncVectorEnv runs N envs in separate processes
    # (dodges the GIL so the Rust env work runs truly in parallel). Set near your
    # physical core count for the biggest speedup.
    p.add_argument("--n-envs", type=int, default=8)
    p.add_argument("--vec-backend", type=str, default="async", choices=["async", "sync"])
    p.add_argument("--seed", type=int, default=42)
    p.add_argument("--save-path", type=str, default="models/dqn_value.pt")
    return p.parse_args()


# Module-level (picklable for AsyncVectorEnv "spawn" workers). Afterstate reward
# scale (nuno-faria): the value net learns board quality, so alive/death stay
# small; lines^2 * clear_width is the objective.
def make_gym_env(max_steps: int):
    import tetris_env  # noqa: F401  ensure "Tetris-v0" is registered in the worker

    return gym.make(
        "Tetris-v0", max_steps=max_steps, alive=1.0, death_penalty=1.0, clear_width=10.0
    )


def build_vec_env(args: argparse.Namespace):
    thunks = [functools.partial(make_gym_env, args.max_steps) for _ in range(args.n_envs)]
    backend = AsyncVectorEnv if args.vec_backend == "async" else SyncVectorEnv
    return backend(thunks)


def train(args: argparse.Namespace) -> ValueMLP:
    set_seed(args.seed)
    if args.device == "auto":
        device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    else:
        device = torch.device(args.device)
    print(f"Using {device} device | {args.n_envs} envs ({args.vec_backend})")

    n = args.n_envs
    venv = build_vec_env(args)
    venv.reset(seed=args.seed)
    # Per-env current candidates: list of (actions, feats) from each parallel env.
    cur = list(venv.call("afterstate_features"))

    online = ValueMLP().to(device)
    target = ValueMLP().to(device)
    target.load_state_dict(online.state_dict())
    optimizer = torch.optim.Adam(online.parameters(), lr=args.lr)
    replay = ReplayBuffer(args.replay_size)

    recent_rew: deque[float] = deque(maxlen=50)
    recent_len: deque[float] = deque(maxlen=50)
    recent_lines: deque[int] = deque(maxlen=50)

    # gymnasium NEXT_STEP autoreset: the step AFTER a sub-env terminates resets it
    # and ignores that step's action. Track a per-env flag to skip that bogus step.
    autoreset = [False] * n
    ep_rew = [0.0] * n
    ep_len = [0] * n
    ep_lines = [0] * n
    episodes_done = 0
    update_count = 0
    # Each iteration collects ~n_envs new transitions. Do n_envs/train_interval
    # gradient steps per iteration so the updates-per-env-step ratio matches the
    # single-env run (1 update per train_interval steps) regardless of n_envs —
    # otherwise more parallel envs silently dilutes the training signal.
    updates_per_iter = max(1, n // args.train_interval)

    try:
        while episodes_done < args.episodes:
            eps = max(args.epsilon_min, epsilon_at(episodes_done, args.epsilon_decay_episodes, 1.0))
            actions = np.zeros(n, dtype=np.int64)
            chosen: list[np.ndarray | None] = [None] * n
            for i in range(n):
                if autoreset[i]:
                    continue
                a_i, c_i = pick_action(online, cur[i][0], cur[i][1], eps, device)
                actions[i] = a_i
                chosen[i] = c_i

            _obs, rews, terms, truncs, infos = venv.step(actions)
            nxt = list(venv.call("afterstate_features"))
            total_lines = infos.get("total_lines")
            tl_mask = infos.get("_total_lines")

            for i in range(n):
                if autoreset[i]:
                    # This step just reset env i (action ignored) — start fresh, no push.
                    autoreset[i] = False
                    cur[i] = nxt[i]
                    ep_rew[i] = 0.0
                    ep_len[i] = 0
                    ep_lines[i] = 0
                    continue

                done = bool(terms[i] or truncs[i])
                next_feats = (
                    np.zeros((0, OBS_DIM), dtype=np.float32)
                    if done
                    else np.asarray(nxt[i][1], dtype=np.float32)
                )
                replay.push(Transition(chosen[i], float(rews[i]), next_feats, done))
                ep_rew[i] += float(rews[i])
                ep_len[i] += 1
                if total_lines is not None and tl_mask is not None and tl_mask[i]:
                    ep_lines[i] = int(total_lines[i])

                if done:
                    recent_rew.append(ep_rew[i])
                    recent_len.append(ep_len[i])
                    recent_lines.append(ep_lines[i])
                    episodes_done += 1
                    autoreset[i] = True  # next step resets this env
                    if episodes_done % 50 == 0:
                        print(
                            f"ep {episodes_done:5d} | eps {eps:.3f} | "
                            f"rew {np.mean(recent_rew):7.2f} | len {np.mean(recent_len):6.1f} | "
                            f"lines/ep {np.mean(recent_lines):6.2f}"
                        )
                else:
                    cur[i] = nxt[i]

            if len(replay) >= max(args.train_start, args.batch_size):
                for _ in range(updates_per_iter):
                    train_step(online, target, replay.sample(args.batch_size), optimizer, args.gamma, device)
                    update_count += 1
                    # Polyak soft update: smooth target network transition
                    for tp, op in zip(target.parameters(), online.parameters()):
                        tp.data.mul_(1 - TAU).add_(op.data, alpha=TAU)
    finally:
        venv.close()

    os.makedirs(os.path.dirname(args.save_path) or ".", exist_ok=True)
    torch.save(online.state_dict(), args.save_path)
    print(f"Saved value net to {args.save_path}")
    return online


def main() -> None:
    train(parse_args())


if __name__ == "__main__":
    main()
