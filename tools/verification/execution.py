"""Shared command execution, filtered diagnostics and persistent logs."""
import os
import re
import subprocess
import sys
import tempfile
import time
from collections import deque
from pathlib import Path
from repository import REPO_ROOT


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


def run_cmd(cmd, cwd=REPO_ROOT, env=None, log_level="warn"):
    print(f"[CMD] {subprocess.list2cmdline(cmd)}", flush=True)
    log_dir = REPO_ROOT / ".agent/tmp/verify"
    log_dir.mkdir(parents=True, exist_ok=True)
    start = time.time()
    with tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", dir=log_dir,
        prefix=f"{Path(cmd[0]).stem}-", suffix=".log", delete=False,
    ) as log:
        log_path = Path(log.name)
        log.write(f"Command: {subprocess.list2cmdline(cmd)}\n")
        log.flush()
        tail = deque(maxlen=12)
        context_left = 0
        error_seen = False
        with subprocess.Popen(cmd, cwd=cwd, env=env, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT, text=True, encoding="utf-8",
                              errors="replace") as process:
            for line in process.stdout:
                log.write(line)
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
    elapsed = time.time() - start
    if code:
        if log_level != "full" and not error_seen:
            print("[DETAIL] Unclassified command failure; last log lines:", flush=True)
            for line in tail:
                print(line.rstrip()[:300], flush=True)
        print(f"[LOG] {log_path.relative_to(REPO_ROOT).as_posix()}", flush=True)
    return code, elapsed

def execute_command(command, overrides, log_level="warn"):
    env = os.environ.copy()
    env["PYTHONIOENCODING"] = "utf-8"
    env["PYTHONUNBUFFERED"] = "1"
    env["VERIFICATION_LOG_LEVEL"] = log_level
    env.update(overrides)
    if overrides:
        print(f"[ENV] {' '.join(f'{key}={value}' for key, value in overrides.items())}", flush=True)
    try:
        code, elapsed = run_cmd(command, env=env, log_level=log_level)
    except OSError as error:
        return False, f"Cannot launch command: {error}", 0
    detail = f"command exited with code {code}"
    return code == 0, detail, elapsed
