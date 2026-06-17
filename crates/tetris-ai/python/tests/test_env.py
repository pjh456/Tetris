from __future__ import annotations

import gymnasium as gym
import numpy as np
from gymnasium.utils.env_checker import check_env

import tetris_env


def unwrap_env(env):
    return env.unwrapped


def test_make_env():
    env = gym.make("Tetris-v0")
    try:
        assert isinstance(unwrap_env(env), tetris_env.TetrisEnv)
    finally:
        env.close()


def test_action_space(env):
    assert isinstance(env.action_space, gym.spaces.Discrete)
    assert env.action_space.n == 40


def test_observation_space(env):
    assert isinstance(env.observation_space, gym.spaces.Box)
    assert env.observation_space.shape == (244,)
    assert env.observation_space.dtype == np.float32
    assert np.all(env.observation_space.low == 0.0)
    assert np.all(env.observation_space.high == 1.0)


def test_reset_obs_shape(env):
    obs, _ = env.reset(seed=42)
    assert obs.shape == (244,)


def test_reset_info(env):
    _, info = env.reset(seed=42)
    assert isinstance(info, dict)
    assert info["initial_lines"] == 0
    assert info["initial_score"] == 0


def test_step_returns(env):
    env.reset(seed=42)
    result = env.step(0)
    assert len(result) == 5


def test_step_obs_shape(env):
    env.reset(seed=42)
    obs, *_ = env.step(0)
    assert obs.shape == (244,)


def test_step_reward_is_float(env):
    env.reset(seed=42)
    _, reward, *_ = env.step(0)
    assert isinstance(reward, float)


def test_step_terminated_is_bool(env):
    env.reset(seed=42)
    _, _, terminated, _, _ = env.step(0)
    assert isinstance(terminated, bool)


def test_step_truncated_is_bool(env):
    env.reset(seed=42)
    _, _, _, truncated, _ = env.step(0)
    assert isinstance(truncated, bool)


def test_step_info_is_dict(env):
    env.reset(seed=42)
    *_, info = env.step(0)
    assert isinstance(info, dict)


def test_determinism():
    first = gym.make("Tetris-v0")
    second = gym.make("Tetris-v0")
    try:
        first_obs, _ = first.reset(seed=42)
        second_obs, _ = second.reset(seed=42)
        np.testing.assert_array_equal(first_obs, second_obs)
    finally:
        first.close()
        second.close()


def test_action_mask(env):
    env.reset(seed=42)
    mask = unwrap_env(env).action_mask()
    assert mask.shape == (40,)
    assert mask.dtype == bool


def test_action_mask_has_legal_action(env):
    env.reset(seed=42)
    mask = unwrap_env(env).action_mask()
    assert int(mask.sum()) > 0


def test_info_keys(env):
    env.reset(seed=42)
    *_, info = env.step(0)
    expected = {
        "lines_cleared",
        "score",
        "combo",
        "max_combo",
        "perfect_clear",
        "damage",
        "damage_sent",
        "damage_received",
        "total_damage",
        "total_lines",
    }
    assert expected <= set(info)
    assert "tspin_count" not in info
    assert "t_spin_count" not in info


def test_gym_check_env():
    env = tetris_env.TetrisEnv(max_steps=100)
    check_env(env)


def test_seeded_first_step_is_deterministic():
    first = gym.make("Tetris-v0")
    second = gym.make("Tetris-v0")
    try:
        first.reset(seed=42)
        second.reset(seed=42)
        first_result = first.step(0)
        second_result = second.step(0)
        np.testing.assert_array_equal(first_result[0], second_result[0])
        assert first_result[1:] == second_result[1:]
    finally:
        first.close()
        second.close()
