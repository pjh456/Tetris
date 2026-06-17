from __future__ import annotations

import gymnasium as gym
import pytest

import tetris_env  # noqa: F401


@pytest.fixture
def env():
    test_env = gym.make("Tetris-v0", max_steps=1_000)
    yield test_env
    test_env.close()
