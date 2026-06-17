#!/usr/bin/env python3
"""Train PPO agent on Tetris-v0 environment."""

from __future__ import annotations

import argparse
import os
from collections.abc import Callable

import gymnasium as gym
from stable_baselines3 import PPO
from stable_baselines3.common.callbacks import BaseCallback
from stable_baselines3.common.vec_env import SubprocVecEnv, VecMonitor

import tetris_env  # noqa: F401


class ActionMaskCallback(BaseCallback):
    """Keep action-mask hook visible for future mask-aware policies."""

    def _on_step(self) -> bool:
        return True


def make_env(rank: int, max_steps: int = 10_000) -> Callable[[], gym.Env]:
    def _init() -> gym.Env:
        env = gym.make("Tetris-v0", max_steps=max_steps)
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
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--max-steps", type=int, default=10_000)
    return parser.parse_args()


def main() -> None:
    args = parse_args()

    env_fns = [make_env(rank, max_steps=args.max_steps) for rank in range(args.n_envs)]
    env = SubprocVecEnv(env_fns, start_method="spawn")
    env = VecMonitor(env)

    model = PPO(
        "MlpPolicy",
        env,
        learning_rate=args.learning_rate,
        n_steps=args.n_steps,
        batch_size=args.batch_size,
        verbose=1,
        seed=args.seed,
        tensorboard_log="./ppo_tetris_logs/",
    )

    print(f"Training PPO on Tetris-v0 for {args.total_timesteps} timesteps...")
    print(f"Environments: {args.n_envs} parallel")
    model.learn(total_timesteps=args.total_timesteps, callback=ActionMaskCallback())

    os.makedirs("models", exist_ok=True)
    model.save("models/ppo_tetris")
    print("Model saved to models/ppo_tetris.zip")
    env.close()


if __name__ == "__main__":
    main()
