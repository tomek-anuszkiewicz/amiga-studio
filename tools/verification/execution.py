"""Shared command execution, filtered diagnostics and persistent logs."""
from contextlib import ExitStack
import json
import os
import re
import subprocess
import sys
import tempfile
import time
from collections import deque
from pathlib import Path
from case_report import CONTEXT
from repository import REPO_ROOT
from run_rust_tests import is_rust_test
from unused_dependencies import UnusedDependencies


COMMAND_LOG = "AMIGA_TEST_COMMAND_LOG"
VERIFIER_LOG_PATH = "AMIGA_VERIFIER_LOG_PATH"
ENGLISH_SCANNER = "tools/verification/checks/check_english.py"
ENGLISH_REPORT_MARKER = "[INFO] English scan report: "


def open_command_log(stack, path):
    if not path:
        return None
    try:
        return stack.enter_context(Path(path).open("x", encoding="utf-8"))
    except OSError as error:
        print(f"[ERROR] Case report command log is unavailable: {error}", flush=True)
        return None


def mirror_output(stream, line):
    if stream:
        try:
            stream.write(line)
            stream.flush()
        except OSError as error:
            print(f"[ERROR] Case report command log is incomplete: {error}", flush=True)
            return None
    return stream


def color_label(label, color, stream=None):
    """Color console labels while keeping redirected output plain."""
    stream = sys.stdout if stream is None else stream
    if not stream.isatty() or "NO_COLOR" in os.environ or os.environ.get("TERM") == "dumb":
        return label
    if os.name == "nt":
        import ctypes
        import msvcrt

        try:
            handle = ctypes.c_void_p(msvcrt.get_osfhandle(stream.fileno()))
            mode = ctypes.c_ulong()
            kernel = ctypes.windll.kernel32
            if not kernel.GetConsoleMode(handle, ctypes.byref(mode)):
                return label
            if not kernel.SetConsoleMode(handle, mode.value | 0x0004):
                return label
        except (AttributeError, OSError, ValueError):
            return label
    return f"\033[{color}m{label}\033[0m"


def result_label(ok, stream=None):
    return color_label("[PASS]" if ok else "[FAIL]", "32" if ok else "31", stream)


def print_info(message):
    print(f"{color_label('[INFO]', '33')} {message}", flush=True)


def diagnostic_level(line):
    """Recognize conventional diagnostic markers in heterogeneous tool output."""
    text = re.sub(r"\x1b\[[0-9;]*m", "", line).strip()
    if text.startswith("[INFO]"):
        return "info"
    if re.search(r"\[(?:FAIL|ERROR|ERR|FATAL)\]|^error(?:\[[^]]+\])?:|^fatal:|"
                 r"^FAIL:|^FAILED\b|^failures:|\sFAILED$|\bpanicked\b|^Traceback\b",
                 text, re.IGNORECASE):
        return "error"
    if re.search(r"\[(?:WARN|WARNING|ANOMALY)\]|^warning(?:\[[^]]+\])?:", text, re.IGNORECASE):
        return "warn"
    return None


def valid_english_entry(entry):
    match entry:
        case {"path": str(path), "spelling": str(spelling), "term": str(term)} if path and spelling and term:
            if entry.get("type") == "word_list":
                line = entry.get("line")
                return type(line) is int and line > 0
            if entry.get("type") == "file_word":
                owner = entry.get("owner")
                position = entry.get("position")
                return isinstance(owner, str) and bool(owner) and type(position) is int and position >= 0
    return False


def valid_english_finding(finding):
    match finding:
        case {"kind": "unknown_word", "path": str(path), "line": line, "word": str(word), "preview": str()}:
            return bool(path and word) and type(line) is int and line > 0
        case {"kind": "unused_entry_candidate", "entry": entry, "reason": str(reason)}:
            return bool(reason) and valid_english_entry(entry)
        case {"kind": "scan_error", "phase": str(phase), "diagnostic": str(diagnostic)}:
            path = finding.get("path")
            return bool(phase and diagnostic) and (path is None or isinstance(path, str) and bool(path))
    return False


def requested_english_files(root, args):
    paths = (Path(os.path.abspath(root / arg)) for arg in args)
    return [(path.relative_to(root) if path.is_relative_to(root) else path).as_posix() for path in paths]


def validate_english_report(cwd, args, code, locators):
    if len(locators) != 1:
        raise ValueError(f"Expected one English scan report locator, found {len(locators)}")
    locator, separator, summary = locators[0].removeprefix(ENGLISH_REPORT_MARKER).partition(" | ")
    if not separator or not summary or not re.fullmatch(r"\.agent/tmp/english/[^/\\:]+\.json", locator):
        raise ValueError(f"Invalid English scan report locator: {locator}")
    root = Path(cwd).resolve()
    path = root / locator
    if path.resolve() != path:
        raise ValueError(f"Invalid English scan report location: {locator}")
    report = json.loads(path.read_text(encoding="utf-8"))
    removal = args[:1] == ["--removals"]
    file_args = args[1:] if args[:1] == ["--"] else args
    requested_files = [] if removal else requested_english_files(root, file_args)
    scope = "files" if requested_files else "repository"
    if not isinstance(report, dict) or report.get("scope") != scope or report.get("requested_files") != requested_files:
        raise ValueError(f"English scan report selection does not match the command: {locator}")
    targets = report.get("target_count")
    scanned = report.get("scanned_count")
    complete = report.get("complete")
    findings = report.get("findings")
    if (type(targets) is not int or type(scanned) is not int or not 0 <= scanned <= targets
            or type(complete) is not bool or complete != (code != 2) or (complete and scanned != targets)
            or not isinstance(findings, list) or not all(map(valid_english_finding, findings))):
        raise ValueError(f"Invalid English scan report contents: {locator}")
    if code not in (0, 1, 2) or type(report.get("exit_code")) is not int or report["exit_code"] != code:
        raise ValueError(f"English scan report status does not match scanner exit code {code}: {locator}")
    validation = report.get("validation")
    if removal:
        if (not isinstance(validation, dict) or type(validation.get("accepted")) is not bool
                or (validation["accepted"] and not complete)):
            raise ValueError(f"Invalid English removal decision: {locator}")
    elif "validation" in report:
        raise ValueError(f"Unexpected English removal decision: {locator}")
    kinds = {finding["kind"] for finding in findings}
    if "scan_error" in kinds:
        expected_code = 2
    elif "unknown_word" in kinds or (removal and not validation["accepted"]):
        expected_code = 1
    else:
        expected_code = 0
    if code != expected_code:
        raise ValueError(f"English scan report findings do not match scanner exit code {code}: {locator}")


def run_cmd(cmd, cwd=None, env=None, log_level="warn", log_dir=None, native=None, check_unused=False):
    cwd = REPO_ROOT if cwd is None else cwd
    print(f"[CMD] {subprocess.list2cmdline(cmd)}", flush=True)
    isolated_log = log_dir is not None
    log_dir = Path(log_dir) if log_dir is not None else REPO_ROOT / ".agent/tmp/verify"
    log_dir.mkdir(parents=True, exist_ok=True)
    start = time.time()
    detail = None
    with ExitStack() as stack:
        log = stack.enter_context(tempfile.NamedTemporaryFile(
            mode="w", encoding="utf-8", dir=log_dir,
            prefix=f"{Path(cmd[0]).stem}-", suffix=".log", delete=False,
        ))
        mirror = open_command_log(stack, env.get(COMMAND_LOG) if env else None)
        mirror = mirror_output(mirror, f"Command: {subprocess.list2cmdline(native or cmd)}\n")
        log_path = Path(log.name)
        log.write(f"Command: {subprocess.list2cmdline(cmd)}\n")
        log.flush()
        tail = deque(maxlen=12)
        context_left = 0
        error_seen = False
        exact_test_count = None
        english = cmd[1:2] == [ENGLISH_SCANNER]
        english_locators = []
        coverage = cmd[:3] == ["cargo", "llvm-cov", "nextest"]
        unused = UnusedDependencies(coverage) if check_unused or coverage else None
        child_env = {**(os.environ if env is None else env), VERIFIER_LOG_PATH: str(log_path)}
        with subprocess.Popen(native or cmd, cwd=cwd, env=child_env, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT, text=True, encoding="utf-8",
                              errors="replace") as process:
            for line in process.stdout:
                log.write(line)
                mirror = mirror_output(mirror, line)
                if unused:
                    unused.consume(line)
                if english and line.startswith(ENGLISH_REPORT_MARKER):
                    english_locators.append(line.rstrip())
                    context_left = 0
                    continue
                if cmd[:2] == ["cargo", "test"] and "--exact" in cmd:
                    count = re.fullmatch(r"running (\d+) tests?", line.strip())
                    if count:
                        exact_test_count = int(count.group(1))
                tail.append(line)
                level = diagnostic_level(line)
                show = level == "error" or (level == "warn" and log_level == "warn")
                error_seen = error_seen or level == "error"
                if level == "info":
                    print_info(line.strip()[len("[INFO]"):].strip())
                    context_left = 0
                elif log_level == "full":
                    print(line, end="", flush=True)
                elif show:
                    print(line.rstrip()[:300], flush=True)
                    context_left = 12
                elif level:
                    context_left = 0
                elif context_left and (not line.strip() or line[0].isspace()):
                    print(line.rstrip()[:300], flush=True)
                    context_left -= 1
                else:
                    context_left = 0
            code = process.wait()
        if unused:
            try:
                findings = unused.findings(cwd, cmd)
            except (OSError, RuntimeError, ValueError) as error:
                findings = []
                diagnostics = [f"[ERROR] Cannot identify the selected dependency declarations: {error}"]
            else:
                diagnostics = [f"[FAIL] Unused dependency {finding}" for finding in findings]
            for diagnostic in diagnostics:
                print(diagnostic, flush=True)
                log.write(diagnostic + "\n")
                mirror = mirror_output(mirror, diagnostic + "\n")
            if diagnostics:
                error_seen = True
                result = f"command exited with code {code}"
                evidence = "unused dependencies: " + ", ".join(findings) if findings else diagnostics[0]
                detail = f"{result} | {evidence}"
                code = code or 1
        if english:
            try:
                validate_english_report(cwd, cmd[2:], code, english_locators)
            except (OSError, ValueError, RuntimeError) as error:
                detail = f"Cannot use English scan report: {error} (scanner exit code {code})"
                diagnostic = f"[ERROR] {detail}\n"
                log.write(diagnostic)
                mirror_output(mirror, diagnostic)
                error_seen = True
                code = 2
            else:
                print_info(english_locators[0].removeprefix("[INFO] "))
    if cmd[:2] == ["cargo", "test"] and "--exact" in cmd and code == 0 and exact_test_count != 1:
        print(f"[FAIL] --exact selected {exact_test_count or 0} tests; expected one.", flush=True)
        error_seen = True
        code = 1
    elapsed = time.time() - start
    if code:
        if log_level != "full" and not error_seen:
            print("[DETAIL] Unclassified command failure; last log lines:", flush=True)
            for line in tail:
                print(line.rstrip()[:300], flush=True)
    if code or isolated_log:
        try:
            display_path = log_path.relative_to(REPO_ROOT).as_posix()
        except ValueError:
            display_path = str(log_path)
        print(f"[LOG] {display_path}", flush=True)
    return code, detail or f"command exited with code {code}", elapsed

def execute_command(command, overrides, log_level="warn", log_dir=None, check_unused=False):
    env = os.environ.copy()
    env["PYTHONIOENCODING"] = "utf-8"
    env["PYTHONUNBUFFERED"] = "1"
    env["VERIFICATION_LOG_LEVEL"] = log_level
    env.update(overrides)
    visible = {key: value for key, value in overrides.items()
               if key not in (*CONTEXT.values(), COMMAND_LOG)}
    if visible:
        print(f"[ENV] {' '.join(f'{key}={value}' for key, value in visible.items())}", flush=True)
    native = None
    if env.get(CONTEXT["parent_invocation_id"]):
        if is_rust_test(command):
            runner = Path(__file__).with_name("run_rust_tests.py")
            native = [sys.executable, str(runner), "--", *command]
        elif command[1:2] == ["tools/verification/checks/check_architecture.py"]:
            runner = Path(__file__).with_name("run_unittest.py")
            native = [sys.executable, str(runner), "--architecture", *command[2:]]
    try:
        options = {"env": env, "log_level": log_level, "log_dir": log_dir}
        if native:
            options["native"] = native
        if check_unused:
            options["check_unused"] = True
        return run_cmd(command, **options)
    except OSError as error:
        code = 2 if command[1:2] == [ENGLISH_SCANNER] else 1
        return code, f"Cannot launch command: {error}", 0
