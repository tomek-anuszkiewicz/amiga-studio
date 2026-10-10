"""Sentence-style command grammar; execution settings are shared options."""
import argparse


EXAMPLES = """Examples (run from the repository root):
  python tools/verification/verify.py commit
  python tools/verification/verify.py commit singlestep dma --plan
  python tools/verification/verify.py test rust crate cpu
  python tools/verification/verify.py test rust crate test_runner suite test_singlestep case test_sub
  python tools/verification/verify.py test python directory tools/bootstrap/tests
  python tools/verification/verify.py test runner singlestep case test_add_b sample 500
  python tools/verification/verify.py check architecture case test_file_size_limits
  python tools/verification/verify.py runner summary --log-level full

Execution options --plan and --log-level full|warn|error may appear anywhere
before a runner's -- separator. Runner options after -- belong to that program.
Focused commands do not run completion gates or coverage. Commit always runs
ordinary crate coverage, quality gates, standard harness and Python contracts.
Milestone runs the language audit; domain skills retain their own checks.
"""


def keyword_values(words, allowed):
    """Read unique keyword/value pairs without silently ignoring unknown words."""
    result = {}
    if len(words) % 2:
        raise ValueError("Expected keyword/value pairs: " + ", ".join(allowed))
    for key, value in zip(words[::2], words[1::2]):
        if key not in allowed or key in result:
            raise ValueError(f"Unknown or repeated selector: {key}")
        if not value.strip():
            raise ValueError(f"Empty value for {key}")
        result[key] = value
    return result


def parser():
    root = argparse.ArgumentParser(description="Repository verification and diagnostics.",
                                   epilog=EXAMPLES, formatter_class=argparse.RawDescriptionHelpFormatter)
    root.add_argument("--plan", action="store_true", help="Preview commands without executing them")
    root.add_argument("--log-level", choices=("full", "warn", "error"), default="warn",
                      help="Child output detail; every run saves full logs")
    commands = root.add_subparsers(dest="command", required=True)
    for name in ("commit", "milestone"):
        leaf = commands.add_parser(name, help=f"Run the {name} completion profile")
        leaf.add_argument("sets", nargs="*", metavar="singlestep|dma")
    tests = commands.add_parser("test", help="Run focused Rust, Python or harness tests")
    kinds = tests.add_subparsers(dest="kind", required=True)
    rust = kinds.add_parser("rust", help="[unit|integration|all|workspace] or crate NAME [suite NAME] [case FILTER]")
    rust.add_argument("words", nargs="*", metavar="SELECTOR")
    python = kinds.add_parser("python", help="All tool contracts or directory PATH [file PATTERN] [case FILTER]")
    python.add_argument("words", nargs="*", metavar="SELECTOR")
    runner = kinds.add_parser("runner", help="[standard|harness|silicon|singlestep|dma] [case FILTER] [sample N|full]")
    runner.add_argument("words", nargs="*", metavar="SELECTOR")
    checks = commands.add_parser("check", help="Run an individual quality or compilation check")
    checks.add_argument("name", choices=("all", "architecture", "english", "code", "hardware",
                                         "formatting", "clippy", "build"))
    checks.add_argument("words", nargs="*", metavar="SELECTOR")
    runs = commands.add_parser("runner", help="Reports, hardware runs, benchmarks, GUI captures and profiling")
    runs.add_argument("action", choices=("build", "summary", "diff", "suite", "bench", "chipset", "vamiga", "gui", "profile", "analyze"))
    runs.add_argument("words", nargs=argparse.REMAINDER)
    return root


def parse_args(argv):
    root = parser()
    if not argv:
        root.print_help()
        return root, None
    # Strip common options before parsing the sentence, including after subcommands.
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--plan", action="store_true")
    common.add_argument("--log-level", choices=("full", "warn", "error"))
    separator = argv.index("--") if "--" in argv else len(argv)
    settings, words = common.parse_known_args(argv[:separator])
    args = root.parse_args(words + argv[separator:])
    report = args.command == "runner" and args.action in ("summary", "diff", "suite", "analyze", "profile")
    args.plan = settings.plan
    args.log_level = settings.log_level or ("full" if report else "warn")
    return root, args
