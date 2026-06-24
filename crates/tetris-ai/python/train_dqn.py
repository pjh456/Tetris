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
import os
import random
from collections import deque
from dataclasses import dataclass

import numpy as np
import torch
import torch.nn as nn
from tetris_ai import TetrisEnv

OBS_DIM = 61


def set_seed(seed: int) -> None:
    """Full reproducibility (pytorch-patterns)."""
    torch.manual_seed(seed)
    torch.cuda.manual_seed_all(seed)
    np.random.seed(seed)
    random.seed(seed)


class ValueMLP(nn.Module):
    """Scores a board afterstate. Input: (batch, 61). Output: (batch, 1)."""

    def __init__(self, input_dim: int = OBS_DIM, hidden: int = 64) -> None:
        super().__init__()
        # Tanh (not ReLU) to match tetris-infer MlpPolicy::forward, which hardcodes
        # tanh between layers — keeps Rust/Python parity (08-11) exact.
        self.net = nn.Sequential(
            nn.Linear(input_dim, hidden),
            nn.Tanh(),
            nn.Linear(hidden, hidden),
            nn.Tanh(),
            nn.Linear(hidden, 1),
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        # x: (batch, 61) -> (batch, 1)
        return self.net(x)


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
    torch.nn.utils.clip_grad_norm_(online.parameters(), max_norm=10.0)
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
    p.add_argument("--target-sync", type=int, default=500)
    p.add_argument("--train-start", type=int, default=1000)
    p.add_argument("--seed", type=int, default=42)
    p.add_argument("--save-path", type=str, default="models/dqn_value.pt")
    return p.parse_args()


def make_env(max_steps: int) -> TetrisEnv:
    # Afterstate reward scale (nuno-faria): the value net learns board quality, so
    # alive/death stay small; lines^2 * clear_width is the objective.
    return TetrisEnv(max_steps=max_steps, alive=1.0, death_penalty=1.0, clear_width=10.0)


def train(args: argparse.Namespace) -> ValueMLP:
    set_seed(args.seed)
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Using {device} device")

    env = make_env(args.max_steps)
    online = ValueMLP().to(device)
    target = ValueMLP().to(device)
    target.load_state_dict(online.state_dict())
    optimizer = torch.optim.Adam(online.parameters(), lr=args.lr)
    replay = ReplayBuffer(args.replay_size)

    global_step = 0
    total_decay_steps = args.episodes * 200  # rough horizon for epsilon anneal
    recent_rew: deque[float] = deque(maxlen=50)
    recent_len: deque[float] = deque(maxlen=50)
    recent_lines: deque[int] = deque(maxlen=50)

    for episode in range(args.episodes):
        env.reset(args.seed + episode)
        actions, feats = env.afterstate_features()
        ep_rew = 0.0
        ep_len = 0
        last_lines = 0
        while len(actions) > 0:
            eps = epsilon_at(global_step, total_decay_steps, args.epsilon_end_frac)
            action, chosen = pick_action(online, actions, feats, eps, device)
            _obs, reward, terminated, truncated, info = env.step(action)
            done = bool(terminated or truncated)
            next_actions, next_feats = (np.empty(0), np.zeros((0, OBS_DIM), dtype=np.float32))
            if not done:
                next_actions, next_feats = env.afterstate_features()
            replay.push(Transition(chosen, float(reward), np.asarray(next_feats, dtype=np.float32), done))

            ep_rew += float(reward)
            ep_len += 1
            last_lines = int(info.get("total_lines", last_lines))
            global_step += 1

            if len(replay) >= args.train_start:
                train_step(online, target, replay.sample(args.batch_size), optimizer, args.gamma, device)
                if global_step % args.target_sync == 0:
                    target.load_state_dict(online.state_dict())

            if done:
                break
            actions, feats = next_actions, next_feats

        recent_rew.append(ep_rew)
        recent_len.append(ep_len)
        recent_lines.append(last_lines)
        if (episode + 1) % 50 == 0:
            print(
                f"ep {episode + 1:5d} | eps {eps:.3f} | "
                f"rew {np.mean(recent_rew):7.2f} | len {np.mean(recent_len):6.1f} | "
                f"lines/ep {np.mean(recent_lines):6.2f}"
            )

    os.makedirs(os.path.dirname(args.save_path) or ".", exist_ok=True)
    torch.save(online.state_dict(), args.save_path)
    print(f"Saved value net to {args.save_path}")
    return online


def main() -> None:
    train(parse_args())


if __name__ == "__main__":
    main()
