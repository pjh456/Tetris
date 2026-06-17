#!/usr/bin/env python3
"""Build tetris-ai Python extension via maturin."""

from __future__ import annotations

import os
import subprocess
import sys


def main() -> None:
    crate_dir = os.path.dirname(os.path.abspath(__file__))
    release = "--release" in sys.argv

    cmd = ["maturin", "develop"]
    if release:
        cmd.append("--release")

    print(f"[build:ai] Running: {' '.join(cmd)} in {crate_dir}")
    result = subprocess.run(cmd, cwd=crate_dir, check=False)
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
