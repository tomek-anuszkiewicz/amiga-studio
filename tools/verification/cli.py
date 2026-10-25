"""Sentence-style command grammar; execution settings are shared options."""
import argparse


COMMAND_TREE = """Command tree (run from the repository root):
  python tools/verification/verify.py
  ├─ commit
  ├─ milestone [singlestep]
  ├─ test
  │  ├─ rust
  │  │  ├─ [unit|integration|all|workspace]
  │  │  └─ crate NAME [suite NAME] [case FILTER]
  │  ├─ python [directory PATH [file PATTERN] [case FILTER]]
  │  └─ runner
  │     ├─ [standard|silicon]
  │     └─ singlestep [case FILTER] [full|sample N]
  ├─ check
  │  ├─ all
  │  ├─ architecture [case FILTER]
  │  ├─ english [files PATH...]
  │  ├─ code
  │  ├─ hardware
  │  ├─ formatting
  │  ├─ clippy
  │  └─ build [crate NAME]
  └─ runner
     ├─ build
     ├─ bench [case ID|mnemonic NAME|opcode XXXX] [duration-ms N] [series N]
     ├─ gui -- OPTIONS
     ├─ profile bench case FAMILY/CASE
     └─ analyze RUN_DIRECTORY

The test rust, test runner and runner bench branches also accept no selector.
Execution options: --plan, --log-level full|warn|error and --results-dir PATH
may appear anywhere among sentence selectors. --exact requires crate NAME,
suite NAME and case NAME for Rust, or case NAME for SingleStep groups.
Only runner gui uses -- to pass native program options.
"""


EXAMPLES = """Examples (run from the repository root):
  python tools/verification/verify.py commit
  python tools/verification/verify.py milestone --plan
  python tools/verification/verify.py test rust crate cpu
  python tools/verification/verify.py test rust crate test_runner suite test_singlestep case test_sub
  python tools/verification/verify.py test python directory tools/bootstrap/tests
  python tools/verification/verify.py test python
  python tools/verification/verify.py test runner singlestep case test_add_b sample 500
  python tools/verification/verify.py test runner singlestep case test_add_b --exact sample 5 --results-dir .agent/tmp/experiments/add
  python tools/verification/verify.py check architecture case test_file_size_limits
  python tools/verification/verify.py runner bench inventory
  python tools/verification/verify.py runner bench case add/add_w_d1_d0 duration-ms 50 series 1
  python tools/verification/verify.py runner bench mnemonic ADD

Focused commands do not run completion gates or coverage. Commit always runs
ordinary crate coverage, quality gates, standard harness and Python contracts.
Without directory selectors, test python runs only contracts affected by changed
tool inputs. Explicit directory selection runs that suite even on a clean tree.
Commit excludes full SingleStep. Milestone runs full SingleStep
and the language audit; domain skills retain their own checks.
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
                                   epilog=COMMAND_TREE + "\n" + EXAMPLES,
                                   formatter_class=argparse.RawDescriptionHelpFormatter)
    root.add_argument("--plan", action="store_true", help="Preview commands without executing them")
    root.add_argument("--log-level", choices=("full", "warn", "error"), default="warn",
                      help="Child output detail; every run saves full logs")
    root.add_argument("--results-dir", metavar="PATH",
                      help="Store verifier logs and SingleStep reports under PATH")
    root.add_argument("--exact", action="store_true",
                      help="Match one Rust suite test or SingleStep group exactly")
    commands = root.add_subparsers(dest="command", required=True)
    for name in ("commit", "milestone"):
        leaf = commands.add_parser(name, help=f"Run the {name} completion profile")
        if name == "milestone":
            leaf.add_argument("sets", nargs="*", metavar="singlestep")
        else:
            leaf.set_defaults(sets=[])
    tests = commands.add_parser("test", help="Run focused Rust, Python or harness tests")
    kinds = tests.add_subparsers(dest="kind", required=True)
    rust = kinds.add_parser("rust", help="[unit|integration|all|workspace] or crate NAME [suite NAME] [case FILTER]")
    rust.add_argument("words", nargs="*", metavar="SELECTOR")
    python = kinds.add_parser("python", help="Changed tool contracts or directory PATH [file PATTERN] [case FILTER]")
    python.add_argument("words", nargs="*", metavar="SELECTOR")
    runner = kinds.add_parser("runner", help="[standard|silicon|singlestep] [case FILTER] [sample N|full]")
    runner.add_argument("words", nargs="*", metavar="SELECTOR")
    checks = commands.add_parser("check", help="Run an individual quality or compilation check")
    checks.add_argument("name", choices=("all", "architecture", "english", "code", "hardware",
                                         "formatting", "clippy", "build"))
    checks.add_argument("words", nargs="*", metavar="SELECTOR")
    runs = commands.add_parser("runner", help="Reports, CPU benchmarks, GUI captures and profiling")
    runs.add_argument("action", choices=("build", "bench", "gui", "profile", "analyze"))
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
    common.add_argument("--results-dir")
    common.add_argument("--exact", action="store_true")
    separator = argv.index("--") if "--" in argv else len(argv)
    settings, words = common.parse_known_args(argv[:separator])
    args = root.parse_args(words + argv[separator:])
    report = args.command == "runner" and args.action in ("analyze", "profile")
    args.plan = settings.plan
    args.log_level = settings.log_level or ("full" if report else "warn")
    args.results_dir = settings.results_dir
    args.exact = settings.exact
    return root, args
