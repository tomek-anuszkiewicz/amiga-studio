"""Check the default Book Review browser on the host."""

from importlib import metadata
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import struct
import sys

from .common import DATA_ERRORS, ProbeError, version_text


BOOK_REVIEW = "tools/bootstrap/book-to-markdown/book_review.py"
EDGE_RELEASES = "https://edgeupdates.microsoft.com/api/products"

# Ask the installed driver which Edge path it uses on this host.
# New bundles export the registry from coreBundle; earlier ones keep it in lib/server.
EDGE_PATH = """
const fs = require("node:fs");
const path = require("node:path");
const home = process.argv[1];
const bundle = path.join(home, "lib", "coreBundle.js");
const exported = fs.existsSync(bundle) ? require(bundle).registry
    : require(path.join(home, "lib", "server", "registry", "index.js"));
const registry = exported.registry ?? exported;
const edge = registry.findExecutable("msedge");
process.stdout.write(JSON.stringify(edge?.executablePath() ?? null));
"""


def edge_path(probe, driver):
    node = driver / ("node.exe" if sys.platform == "win32" else "node")
    command = os.environ.get("PLAYWRIGHT_NODEJS_PATH") or str(node)
    executable = json.loads(probe.run([command, "-e", EDGE_PATH, str(driver / "package")],
                                      env={"NODE_OPTIONS": None, "NODE_PATH": None}))
    if executable is not None and not isinstance(executable, str):
        raise ProbeError("the Playwright registry did not return an Edge path")
    return Path(executable) if executable else None


def four_part_version(text):
    match = re.search(r"(?<![\d.])(\d+\.\d+\.\d+\.\d+)(?![\d.])", text)
    if not match:
        raise ProbeError("the executable or upstream record did not report a four-part browser version")
    return tuple(map(int, match.group(1).split(".")))


def windows_architecture(path):
    with path.open("rb") as stream:
        start = stream.read(64)
        if len(start) != 64 or start[:2] != b"MZ":
            raise ProbeError("Edge's Windows executable header is unrecognized")
        stream.seek(struct.unpack_from("<I", start, 60)[0])
        data = stream.read(6)
    if len(data) != 6 or data[:4] != b"PE\0\0":
        raise ProbeError("Edge's Windows executable architecture is unrecognized")
    return {0x014C: "x86", 0x8664: "x64", 0xAA64: "arm64"}.get(struct.unpack_from("<H", data, 4)[0])


def edge_installed_version(probe, path):
    if sys.platform == "win32":
        powershell = shutil.which("pwsh") or shutil.which("powershell")
        if not powershell:
            raise ProbeError("PowerShell is unavailable to read the Edge version resource")
        command = ["-NoProfile", "-NonInteractive", "-Command",
                   "(Get-Item -LiteralPath $env:AMIGA_EDGE_PROBE_PATH).VersionInfo.ProductVersion"]
        version = probe.run([powershell, *command], env={"AMIGA_EDGE_PROBE_PATH": str(path)})
        return four_part_version(version), "windows", windows_architecture(path)
    if sys.platform == "darwin":
        with (path.parents[1] / "Info.plist").open("rb") as file:
            info = plistlib.load(file)
        return four_part_version(info["CFBundleShortVersionString"]), "macos", "universal"
    if sys.platform == "linux":
        import platform
        architecture = {"x86_64": "x64", "aarch64": "arm64"}.get(platform.machine())
        return four_part_version(probe.run([str(path), "--version"])), "linux", architecture
    raise ProbeError("Edge release distribution for this platform could not be identified")


def check_edge(area, probe, executable):
    if executable is None or not executable.is_file():
        area.record("missing", "Microsoft Edge Stable", observed="not found", declared="msedge",
                    source=BOOK_REVIEW, detail="default Book Review browser")
        return
    installed = None
    try:
        installed, system, architecture = edge_installed_version(probe, executable)
        releases = [four_part_version(row["ProductVersion"])
                    for product in probe.http(EDGE_RELEASES) if product["Product"] == "Stable"
                    for row in product["Releases"] if row["Platform"].lower() == system and
                    row["Architecture"].lower() == architecture]
        if not releases:
            raise ProbeError("Microsoft's Stable list does not identify this platform and architecture")
        area.compare("Microsoft Edge Stable", installed, max(releases), EDGE_RELEASES, declared="msedge")
    except DATA_ERRORS as error:
        area.incomplete("Microsoft Edge Stable", EDGE_RELEASES,
                        error if isinstance(error, ProbeError) else "invalid installed or upstream Edge version data",
                        observed=version_text(installed) if installed is not None else "executable found",
                        declared="msedge")


def check_browsers(area, probe, _tracked):
    try:
        playwright = metadata.distribution("playwright")
        driver = Path(playwright.locate_file("playwright/driver"))
        check_edge(area, probe, edge_path(probe, driver))
    except (*DATA_ERRORS, metadata.PackageNotFoundError) as error:
        area.incomplete("Microsoft Edge Stable", BOOK_REVIEW,
                        error if isinstance(error, ProbeError) else "the selected interpreter's Playwright "
                                                                     "driver could not provide an Edge path",
                        declared="msedge")
