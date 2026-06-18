"""Gymnasium wrapper for the Rust Tetris RL environment."""

from __future__ import annotations

from typing import Any

import gymnasium as gym
import numpy as np
from gymnasium.envs.registration import register
from tetris_ai import TetrisEnv as RustTetrisEnv

OBS_DIM = 61
ACTION_SPACE_SIZE = 40
ENV_ID = "Tetris-v0"


class TetrisEnv(gym.Env):
    """Gymnasium wrapper for Rust Tetris RL environment."""

    metadata = {"render_modes": []}

    def __init__(self, max_steps: int = 10_000, **reward_kwargs: Any) -> None:
        super().__init__()
        self._env = RustTetrisEnv(
            max_steps=max_steps,
            hole_penalty=reward_kwargs.get("hole_penalty", -0.5),
            height_penalty=reward_kwargs.get("height_penalty", -0.3),
            bumpiness_penalty=reward_kwargs.get("bumpiness_penalty", -0.2),
            well_penalty=reward_kwargs.get("well_penalty", -0.3),
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

    def action_mask(self) -> np.ndarray:
        return np.asarray(self._env.action_mask(), dtype=bool)


if ENV_ID not in gym.envs.registry:
    register(id=ENV_ID, entry_point="tetris_env:TetrisEnv")
