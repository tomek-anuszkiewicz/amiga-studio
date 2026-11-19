"""Launch the readied review service with an isolated runtime client home."""

import json
import os
from pathlib import Path
import shutil
import sys


def main():
    try:
        variants = json.loads(os.environ["BOOK_REVIEW_PROTECTED_VARIANTS"])
        if not isinstance(variants, list) or not all(isinstance(item, str) for item in variants):
            raise ValueError("Invalid variant selection")
    except (KeyError, ValueError):
        raise SystemExit("Generate and apply the Book Review mount override before startup") from None

    if source := os.environ.get("BOOK_REVIEW_AUTH"):
        destination = Path(os.environ["CODEX_HOME"]) / "auth.json"
        try:
            with open(source, "rb") as auth:
                descriptor = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
                with os.fdopen(descriptor, "wb") as private:
                    shutil.copyfileobj(auth, private)
        except OSError:
            raise SystemExit("Cannot load the selected runtime Codex authentication file") from None

    script = Path(__file__).with_name("book_review.py")
    os.execv(sys.executable, [sys.executable, str(script), "serve", "--host", "0.0.0.0",
                            "--library", "/library", "--channel", "", "--protected-variants",
                            json.dumps(variants)])


if __name__ == "__main__":
    main()
