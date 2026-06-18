#!/usr/bin/env python3
"""Export SB3 actor weights into the tetris-infer JSON schema."""

from __future__ import annotations

import argparse
import json
import random
from pathlib import Path
from typing import Any

import gymnasium as gym
import numpy as np

import tetris_env  # noqa: F401
from tetris_env import ACTION_SPACE_SIZE, ENV_ID, OBS_DIM

PROJECT_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_MODEL_PATH = PROJECT_ROOT / "models" / "ppo_tetris"
DEFAULT_OUT_PATH = PROJECT_ROOT / "models" / "weights.json"
DEFAULT_FIXTURE_PATH = PROJECT_ROOT / "crates" / "tetris-infer" / "tests" / "fixtures" / "parity.json"
PLACEHOLDER_SEED = 42
HIDDEN_DIM = 64


def export(
    model_path: str | Path = DEFAULT_MODEL_PATH,
    out_path: str | Path = DEFAULT_OUT_PATH,
    fixture_path: str | Path | None = None,
) -> dict[str, Any]:
    model_path = Path(model_path)
    out_path = Path(out_path)
    fixture_path = Path(fixture_path) if fixture_path is not None else None

    weights = load_actor_weights(model_path)
    if weights is None or weights["input_dim"] != OBS_DIM or weights["output_dim"] != ACTION_SPACE_SIZE:
        # Placeholder exists only to unblock downstream load/embed paths until 08-08 retrains.
        weights = build_placeholder_weights()

    write_json(out_path, weights)
    if fixture_path is not None:
        obs = fixture_obs()
        logits = forward_logits(weights, obs)
        write_json(fixture_path, {"obs": obs, "logits": logits, "weights": weights})
    return weights


def load_actor_weights(model_path: Path) -> dict[str, Any] | None:
    if not model_path.exists() and not model_path.with_suffix(".zip").exists():
        return None

    from stable_baselines3 import PPO

    model = PPO.load(model_path)
    layers = []
    for module in list(model.policy.mlp_extractor.policy_net) + [model.policy.action_net]:
        if hasattr(module, "weight"):
            layers.append(
                {
                    "weight": module.weight.detach().cpu().tolist(),
                    "bias": module.bias.detach().cpu().tolist(),
                }
            )
    return schema_from_layers(layers)


def schema_from_layers(layers: list[dict[str, list[Any]]]) -> dict[str, Any]:
    return {
        "input_dim": len(layers[0]["weight"][0]),
        "output_dim": len(layers[-1]["bias"]),
        "activation": "tanh",
        "layers": layers,
    }


def build_placeholder_weights() -> dict[str, Any]:
    rng = random.Random(PLACEHOLDER_SEED)
    dims = [OBS_DIM, HIDDEN_DIM, HIDDEN_DIM, ACTION_SPACE_SIZE]
    layers = []
    for in_dim, out_dim in zip(dims, dims[1:]):
        layers.append(
            {
                "weight": [
                    [round(rng.uniform(-0.05, 0.05), 8) for _ in range(in_dim)]
                    for _ in range(out_dim)
                ],
                "bias": [round(rng.uniform(-0.01, 0.01), 8) for _ in range(out_dim)],
            }
        )
    return schema_from_layers(layers)


def fixture_obs() -> list[float]:
    env = gym.make(ENV_ID)
    try:
        obs, _ = env.reset(seed=42)
        return np.asarray(obs, dtype=np.float32).tolist()
    finally:
        env.close()


def forward_logits(weights: dict[str, Any], obs: list[float]) -> list[float]:
    activation = obs
    for layer_index, layer in enumerate(weights["layers"]):
        values = []
        for row, bias in zip(layer["weight"], layer["bias"]):
            values.append(float(bias) + sum(float(weight) * value for weight, value in zip(row, activation)))
        if layer_index + 1 < len(weights["layers"]):
            activation = np.tanh(np.asarray(values, dtype=np.float32)).astype(np.float32).tolist()
        else:
            activation = values
    return activation


def write_json(path: Path, data: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2), encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", type=Path, default=DEFAULT_MODEL_PATH)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT_PATH)
    parser.add_argument("--fixture", type=Path, default=DEFAULT_FIXTURE_PATH)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    export(args.model, args.out, args.fixture)


if __name__ == "__main__":
    main()
