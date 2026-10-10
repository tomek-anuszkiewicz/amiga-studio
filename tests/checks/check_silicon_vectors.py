"""Require external 68000 vectors before CPU verification can count as complete."""

from pathlib import Path
import sys


def main():
    root = Path(__file__).resolve().parents[2]
    vectors = root / "ref_src/SingleStepTests-680x0/68000/v1"
    if not any(vectors.glob("*.json")):
        print("Silicon vectors missing; provision external sources before verification")
        return 1
    print("68000 silicon vectors available")
    return 0


if __name__ == "__main__":
    sys.exit(main())
