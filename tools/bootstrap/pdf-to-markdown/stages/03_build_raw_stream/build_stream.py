#!/usr/bin/env python3
"""
stages/03_build_raw_stream/build_stream.py:
Legacy raw-stream assembly is unavailable until adapted to Stage 02d artifacts.
"""

import argparse
import sys
from pathlib import Path
import yaml

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from conversion.config import UniqueLoader



def build_raw_stream(workspace_dir: Path, config: dict):
    raise ValueError("Stage 03 is unavailable: its segmentation input was removed; "
                     "assembly from Stage 02d md_text/pixel boxes requires a separate redesign")


def main():
    parser = argparse.ArgumentParser(description="Stage 03: Build raw sequential node stream and extract initial assets")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)
    if not config_path.is_file():
        raise FileNotFoundError(f"Stage 03: Config file not found: {config_path}")

    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.load(f, Loader=UniqueLoader)
    if not config or not isinstance(config, dict):
        raise ValueError(f"Stage 03: Config file is empty or invalid: {config_path}")

    build_raw_stream(workspace_dir, config)


if __name__ == "__main__":
    main()
