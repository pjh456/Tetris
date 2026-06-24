#!/usr/bin/env python3
"""Export the afterstate-DQN value net into the tetris-infer JSON schema.

The value net (train_dqn.ValueMLP: 61 -> 64 -> 64 -> 1, Tanh hidden) is a plain
MLP, so it serializes to the same weights.json schema tetris-infer reads — only
output_dim is 1 (a scalar board value) instead of the old 40 action logits.
tetris-infer's MlpPolicy::forward hardcodes tanh between layers, so the value net
uses tanh and the parity fixture matches exactly.
"""

from __future__ import annotations

import argparse
import json
import random
from pathlib import Path
from typing import Any

import gymnasium as gym
import numpy as np
import torch

import tetris_env  # noqa: F401
from tetris_env import ENV_ID, OBS_DIM

PROJECT_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_MODEL_PATH = PROJECT_ROOT / "models" / "dqn_value.pt"
DEFAULT_OUT_PATH = PROJECT_ROOT / "models" / "weights.json"
DEFAULT_FIXTURE_PATH = PROJECT_ROOT / "crates" / "tetris-infer" / "tests" / "fixtures" / "parity.json"
PLACEHOLDER_SEED = 42
HIDDEN_DIM = 64
VALUE_OUTPUT_DIM = 1
# ValueMLP Sequential: Linear @ 0, 2, 4 (Tanh at 1, 3).
LINEAR_INDICES = (0, 2, 4)


def export(
    model_path: str | Path = DEFAULT_MODEL_PATH,
    out_path: str | Path = DEFAULT_OUT_PATH,
    fixture_path: str | Path | None = None,
) -> dict[str, Any]:
    model_path = Path(model_path)
    out_path = Path(out_path)
    fixture_path = Path(fixture_path) if fixture_path is not None else None

    weights = load_value_weights(model_path)
    if weights is None or weights["input_dim"] != OBS_DIM or weights["output_dim"] != VALUE_OUTPUT_DIM:
        # Placeholder unblocks the load/embed/parity paths until 08-12 trains the real net.
        weights = build_placeholder_weights()

    write_json(out_path, weights)
    if fixture_path is not None:
        obs = fixture_obs()
        value = forward_logits(weights, obs)
        write_json(fixture_path, {"obs": obs, "logits": value, "weights": weights})
    return weights


def load_value_weights(model_path: Path) -> dict[str, Any] | None:
    pt_path = model_path if model_path.suffix == ".pt" else model_path.with_suffix(".pt")
    if not pt_path.exists():
        return None

    # state_dict of train_dqn.ValueMLP (keys net.{0,2,4}.weight/bias). Read directly
    # so we do not need to import the training module / rebuild the env.
    state_dict = torch.load(pt_path, map_location="cpu", weights_only=True)
    layers = []
    for index in LINEAR_INDICES:
        weight = state_dict[f"net.{index}.weight"]
        bias = state_dict[f"net.{index}.bias"]
        layers.append({"weight": weight.cpu().tolist(), "bias": bias.cpu().tolist()})
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
    dims = [OBS_DIM, HIDDEN_DIM, HIDDEN_DIM, VALUE_OUTPUT_DIM]
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
