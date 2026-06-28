"""Gymnasium wrapper for the Rust Tetris RL environment."""

from __future__ import annotations

from typing import Any

import gymnasium as gym
import numpy as np
from gymnasium.envs.registration import register
from tetris_ai import TetrisEnv as RustTetrisEnv

OBS_DIM = 71
ACTION_SPACE_SIZE = 40
ENV_ID = "Tetris-v0"


class TetrisEnv(gym.Env):
    """Gymnasium wrapper for Rust Tetris RL environment."""

    metadata = {"render_modes": []}

    def __init__(self, max_steps: int = 10_000, **reward_kwargs: Any) -> None:
        super().__init__()
        # Clear-dominant reward (08-08): lines^2 * clear_width dominates; a SMALL
        # alive bonus breaks the sparse -death plateau (survival forces clearing,
        # since the board tops out in ~50 placements without a clear); game-over
        # pays only -death_penalty (no terminal leak); shaping disabled (ZERO).
        # Defaults mirror the Rust RewardConfig::default() so omitting kwargs matches
        # the crate default (gamma 0.99, alive 0.1, death_penalty 50.0, clear_width 10.0).
        self._env = RustTetrisEnv(
            max_steps=max_steps,
            gamma=reward_kwargs.get("gamma", 0.99),
            alive=reward_kwargs.get("alive", 0.1),
            death_penalty=reward_kwargs.get("death_penalty", 50.0),
            clear_width=reward_kwargs.get("clear_width", 10.0),
        )
        self.action_space = gym.spaces.Discrete(ACTION_SPACE_SIZE)
        self.observation_space = gym.spaces.Box(
            low=0.0,
            high=1.0,
            shape=(OBS_DIM,),
            dtype=np.float32,
        )

    def reset(
        self,
        *,
        seed: int | None = None,
        options: dict[str, Any] | None = None,
    ) -> tuple[np.ndarray, dict[str, Any]]:
        super().reset(seed=seed)
        del options
        obs, info = self._env.reset(seed)
        return np.asarray(obs, dtype=np.float32), dict(info)

    def step(self, action: int) -> tuple[np.ndarray, float, bool, bool, dict[str, Any]]:
        obs, reward, terminated, truncated, info = self._env.step(int(action))
        obs_array = np.asarray(obs, dtype=np.float32)
        assert obs_array.shape == self.observation_space.shape, (
            f"Obs shape mismatch: {obs_array.shape} vs {self.observation_space.shape}"
        )
        return obs_array, float(reward), bool(terminated), bool(truncated), dict(info)

    def afterstate_features(self) -> tuple[np.ndarray, np.ndarray]:
        """Legal action indices + their resulting-board 61-dim features.

        Exposed on the wrapper so gymnasium VectorEnv.call("afterstate_features")
        reaches it in each parallel worker (afterstate-DQN vectorized training).
        """
        actions, feats = self._env.afterstate_features()
        return np.asarray(actions, dtype=np.int64), np.asarray(feats, dtype=np.float32)

    def afterstate_features_with_hold(self) -> tuple[np.ndarray, np.ndarray]:
        """Combined afterstate candidates: 0-39 current piece, 40-79 held piece."""
        actions, feats = self._env.afterstate_features_with_hold()
        return np.asarray(actions, dtype=np.int64), np.asarray(feats, dtype=np.float32)

    def action_mask(self) -> np.ndarray:
        return np.asarray(self._env.action_mask(), dtype=bool)

    def action_masks(self) -> np.ndarray:
        """Mask consumed by sb3-contrib MaskablePPO via ActionMasker."""
        return self.action_mask()


if ENV_ID not in gym.envs.registry:
    register(id=ENV_ID, entry_point="tetris_env:TetrisEnv")
