"""Smoke tests for the afterstate value-DQN trainer (08-10).

Pure-torch tests run without the Rust env. The short end-to-end training run
needs the rebuilt PyO3 extension (afterstate_features) and skips otherwise.
"""

from __future__ import annotations

import argparse

import numpy as np
import torch

from train_dqn import (
    OBS_DIM,
    ReplayBuffer,
    Transition,
    ValueMLP,
    epsilon_at,
    train_step,
)


def test_value_mlp_forward_shape() -> None:
    net = ValueMLP()
    out = net(torch.randn(8, OBS_DIM))
    assert out.shape == (8, 1)


def test_epsilon_anneals() -> None:
    assert epsilon_at(0, 1000, 0.75) == 1.0
    mid = epsilon_at(375, 1000, 0.75)
    assert 0.0 < mid < 1.0
    assert epsilon_at(750, 1000, 0.75) == 0.0
    assert epsilon_at(5000, 1000, 0.75) == 0.0


def test_replay_buffer_caps_at_capacity() -> None:
    buf = ReplayBuffer(3)
    empty_next = np.zeros((0, OBS_DIM), dtype=np.float32)
    for _ in range(5):
        buf.push(Transition(np.zeros(OBS_DIM, dtype=np.float32), 0.0, empty_next, 0.0))
    assert len(buf) == 3


def test_train_step_returns_finite_loss() -> None:
    device = torch.device("cpu")
    online = ValueMLP().to(device)
    target = ValueMLP().to(device)
    target.load_state_dict(online.state_dict())
    opt = torch.optim.Adam(online.parameters(), lr=1e-3)
    rng = np.random.default_rng(0)
    batch: list[Transition] = []
    for _ in range(16):
        chosen = rng.standard_normal(OBS_DIM).astype(np.float32)
        done = bool(rng.integers(0, 2))
        if done:
            nxt = np.zeros((0, OBS_DIM), dtype=np.float32)
        else:
            m = int(rng.integers(1, 6))
            nxt = rng.standard_normal((m, OBS_DIM)).astype(np.float32)
            gamma_n = 0.0 if done else 0.95
            batch.append(Transition(chosen, float(rng.standard_normal()), nxt, gamma_n))
    loss = train_step(online, target, batch, opt, gamma=0.95, device=device)
    assert np.isfinite(loss)


def test_short_train_run_smoke() -> None:
    """End-to-end: needs the rebuilt extension with afterstate_features."""
    try:
        import tetris_ai

        env = tetris_ai.TetrisEnv()
        env.reset(0)
        env.afterstate_features()
    except (ImportError, AttributeError):
        import pytest

        pytest.skip("tetris_ai without afterstate_features; run maturin develop --release")

    from train_dqn import train

    args = argparse.Namespace(
        episodes=2,
        max_steps=200,
        replay_size=500,
        batch_size=16,
        gamma=0.95,
            lr=1e-3,
            hidden=64,
            epsilon_end_frac=0.75,
        epsilon_min=0.02,
        epsilon_decay_episodes=10,
        target_sync=50,
        train_start=16,
        train_interval=1,
        n_step=2,
        polyak_interval=1,
        device="cpu",
        n_envs=2,
        vec_backend="sync",
        seed=1,
        save_path="models/dqn_value_smoke.pt",
        resume=None,
    )
    net = train(args)
    assert net is not None
