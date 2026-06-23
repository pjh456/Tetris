#!/usr/bin/env python3
"""Train a MaskablePPO agent on the Tetris-v0 environment.

Uses sb3-contrib MaskablePPO so illegal-action logits are masked at the sampling
layer (08-07, D4 / RESEARCH Q2). The policy-net structure is unchanged, so the
weights.json export (08-02) and tetris-infer parity (08-01) stay compatible.
"""

from __future__ import annotations

import argparse
import os
from collections.abc import Callable

import gymnasium as gym
from sb3_contrib import MaskablePPO
from sb3_contrib.common.wrappers import ActionMasker
from stable_baselines3.common.vec_env import SubprocVecEnv, VecMonitor, VecNormalize

import tetris_env  # noqa: F401


def mask_fn(env: gym.Env) -> "object":
    """ActionMasker hook — returns the env's current legal-action mask.

    Resolve through `.unwrapped`: gymnasium wrappers (OrderEnforcing/
    PassiveEnvChecker) do not forward the custom `action_masks` method.
    """
    return env.unwrapped.action_masks()


def make_env(rank: int, max_steps: int = 10_000) -> Callable[[], gym.Env]:
    def _init() -> gym.Env:
        env = gym.make("Tetris-v0", max_steps=max_steps)
        env = ActionMasker(env, mask_fn)
        env.reset(seed=42 + rank)
        return gym.wrappers.RecordEpisodeStatistics(env)

    return _init


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--total-timesteps", type=int, default=1_000_000)
    parser.add_argument("--n-envs", type=int, default=4)
    parser.add_argument("--learning-rate", type=float, default=3e-4)
    parser.add_argument("--n-steps", type=int, default=2048)
    parser.add_argument("--batch-size", type=int, default=64)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--max-steps", type=int, default=10_000)
    return parser.parse_args()


def main() -> None:
    args = parse_args()

    env_fns = [make_env(rank, max_steps=args.max_steps) for rank in range(args.n_envs)]
    env = SubprocVecEnv(env_fns, start_method="spawn")
    # VecMonitor BEFORE VecNormalize so logged ep_rew_mean stays on the raw scale
    # (the D11 acceptance bar is raw ep_rew_mean > 0).
    env = VecMonitor(env)
    # Normalize the REWARD only (returns → unit variance) to rescue the critic
    # (explained_variance ≈ 0 / value_loss blowup under the large-scale shaped
    # reward). obs is left raw ([0,1] already) so the exported weights.json stays
    # parity-compatible with tetris-infer (inference does no obs normalization).
    env = VecNormalize(
        env,
        norm_obs=False,
        norm_reward=True,
        clip_reward=10.0,
        gamma=args.gamma,
    )

    # gamma must match the reward-shaping discount (08-07 DEFAULT_GAMMA) so the
    # potential-based shaping term stays policy-invariant (Ng 1999, Pitfall 7).
    model = MaskablePPO(
        "MlpPolicy",
        env,
        learning_rate=args.learning_rate,
        n_steps=args.n_steps,
        batch_size=args.batch_size,
        gamma=args.gamma,
        verbose=1,
        seed=args.seed,
        tensorboard_log="./ppo_tetris_logs/",
    )

    print(f"Training MaskablePPO on Tetris-v0 for {args.total_timesteps} timesteps...")
    print(f"Environments: {args.n_envs} parallel")
    model.learn(total_timesteps=args.total_timesteps)

    os.makedirs("models", exist_ok=True)
    model.save("models/ppo_tetris")
    print("Model saved to models/ppo_tetris.zip")
    env.close()


if __name__ == "__main__":
    main()
