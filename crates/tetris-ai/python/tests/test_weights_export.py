from __future__ import annotations

import json
import shutil
from pathlib import Path

import export_weights
from tetris_env import OBS_DIM

PROJECT_ROOT = Path(__file__).resolve().parents[4]
TEST_OUTPUT_DIR = PROJECT_ROOT / "target" / "pytest_weights_export"


def assert_dim_chain(weights: dict) -> None:
    assert set(weights) == {"input_dim", "output_dim", "activation", "layers"}
    assert weights["input_dim"] == OBS_DIM
    assert weights["output_dim"] == export_weights.VALUE_OUTPUT_DIM
    assert weights["activation"] == "tanh"
    assert weights["layers"]

    expected_input_dim = weights["input_dim"]
    for layer in weights["layers"]:
        assert len(layer["bias"]) == len(layer["weight"])
        assert all(len(row) == expected_input_dim for row in layer["weight"])
        expected_input_dim = len(layer["weight"])
    assert expected_input_dim == weights["output_dim"]


def test_placeholder_weights_match_schema() -> None:
    assert_dim_chain(export_weights.build_placeholder_weights())


def clean_output_dir() -> Path:
    shutil.rmtree(TEST_OUTPUT_DIR, ignore_errors=True)
    TEST_OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    return TEST_OUTPUT_DIR


def test_exported_weights_file_matches_schema() -> None:
    output_dir = clean_output_dir()
    weights_path = output_dir / "weights.json"
    fixture_path = output_dir / "parity.json"
    export_weights.export(output_dir / "missing_model.zip", weights_path, fixture_path)
    weights = json.loads(weights_path.read_text(encoding="utf-8"))
    assert_dim_chain(weights)


def test_parity_fixture_matches_schema() -> None:
    output_dir = clean_output_dir()
    weights_path = output_dir / "weights.json"
    fixture_path = output_dir / "parity.json"
    export_weights.export(output_dir / "missing_model.zip", weights_path, fixture_path)
    fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
    assert len(fixture["obs"]) == OBS_DIM
    assert len(fixture["logits"]) == export_weights.VALUE_OUTPUT_DIM
    assert_dim_chain(fixture["weights"])
