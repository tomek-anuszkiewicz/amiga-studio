#!/usr/bin/env python3
"""Report development dependency updates and installation drift for this checkout."""

import argparse
from pathlib import Path
import sys

# Keep even a plain Python invocation from leaving import caches in this checkout.
if __name__ == "__main__":
    sys.dont_write_bytecode = True

from dependency_updates.browsers import check_browsers
from dependency_updates.cargo import check_crates
from dependency_updates.common import Area, DATA_ERRORS, ProbeError, Probes, Report
from dependency_updates.docker_tools import check_docker
from dependency_updates.node_packages import check_node_packages
from dependency_updates.python_packages import check_python_packages, check_python_runtime
from dependency_updates.setup_tools import check_rust, check_setup


ROOT = Path(__file__).resolve().parents[1]


def check_python(area, probe, tracked):
    check_python_runtime(area, probe)
    check_python_packages(area, probe, tracked)


def report(probe, out):
    output = Report(out)
    output.write("Finding project dependency files...")
    inventory = Area("Project inventory")
    try:
        tracked = probe.tracked_files()
    except ProbeError as error:
        tracked = set()
        inventory.incomplete("Git tracked files", "git ls-files", error)
        output.write("\nProject inventory")
        output.write_area(inventory)

    for name, check in (
        ("Rust toolchains and Cargo tools", check_rust),
        ("Cargo packages", check_crates),
        ("Python and pip", check_python),
        ("Book Review browsers", check_browsers),
        ("Node.js and npm", check_node_packages),
        ("Other project tools", check_setup),
        ("Docker and Qdrant", check_docker),
    ):
        output.start_area(name)
        area = Area(name, progress=output.write)
        try:
            check(area, probe, tracked)
        except DATA_ERRORS as error:
            area.incomplete(name, "development dependency inventory",
                            error if isinstance(error, ProbeError) else "invalid or unavailable local or upstream data")
        output.write_area(area)
    output.finish()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args(argv)
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    report(Probes(ROOT), sys.stdout)
    return 0


if __name__ == "__main__":
    sys.exit(main())
