"""Windows command containment and a handle guardian for interrupted batches."""
import contextlib
import ctypes as c
from ctypes import wintypes as w
import msvcrt
import os
from pathlib import Path
import subprocess
import sys
import time


kernel = c.WinDLL("kernel32", use_last_error=True)


def api(name, result, *arguments):
    function = getattr(kernel, name)
    function.restype = result
    function.argtypes = arguments
    return function


create_job = api("CreateJobObjectW", w.HANDLE, c.c_void_p, w.LPCWSTR)
open_job = api("OpenJobObjectW", w.HANDLE, w.DWORD, w.BOOL, w.LPCWSTR)
close_handle = api("CloseHandle", w.BOOL, w.HANDLE)
query_job = api("QueryInformationJobObject", w.BOOL, w.HANDLE, c.c_int, c.c_void_p, w.DWORD, c.c_void_p)
initialize_attributes = api("InitializeProcThreadAttributeList", w.BOOL, c.c_void_p, w.DWORD,
                            w.DWORD, c.POINTER(c.c_size_t))
update_attribute = api("UpdateProcThreadAttribute", w.BOOL, c.c_void_p, w.DWORD, c.c_size_t,
                       c.c_void_p, c.c_size_t, c.c_void_p, c.c_void_p)
delete_attributes = api("DeleteProcThreadAttributeList", None, c.c_void_p)
duplicate_handle = api("DuplicateHandle", w.BOOL, w.HANDLE, w.HANDLE, w.HANDLE,
                       c.POINTER(w.HANDLE), w.DWORD, w.BOOL, w.DWORD)
get_std_handle = api("GetStdHandle", w.HANDLE, w.DWORD)
wait_handle = api("WaitForSingleObject", w.DWORD, w.HANDLE, w.DWORD)
exit_code = api("GetExitCodeProcess", w.BOOL, w.HANDLE, c.POINTER(w.DWORD))
open_process = api("OpenProcess", w.HANDLE, w.DWORD, w.BOOL, w.DWORD)
process_in_job = api("IsProcessInJob", w.BOOL, w.HANDLE, w.HANDLE, c.POINTER(w.BOOL))
process_image = api("QueryFullProcessImageNameW", w.BOOL, w.HANDLE, w.DWORD,
                    w.LPWSTR, c.POINTER(w.DWORD))
terminate_process = api("TerminateProcess", w.BOOL, w.HANDLE, w.UINT)


class StartupInfo(c.Structure):
    _fields_ = [("cb", w.DWORD), ("reserved", w.LPWSTR), ("desktop", w.LPWSTR),
                ("title", w.LPWSTR), ("x", w.DWORD), ("y", w.DWORD),
                ("width", w.DWORD), ("height", w.DWORD), ("x_chars", w.DWORD),
                ("y_chars", w.DWORD), ("fill", w.DWORD), ("flags", w.DWORD),
                ("show", w.WORD), ("reserved_size", w.WORD), ("reserved_bytes", c.c_void_p),
                ("stdin", w.HANDLE), ("stdout", w.HANDLE), ("stderr", w.HANDLE)]


class StartupInfoEx(c.Structure):
    _fields_ = [("startup", StartupInfo), ("attributes", c.c_void_p)]


class ProcessInfo(c.Structure):
    _fields_ = [("process", w.HANDLE), ("thread", w.HANDLE), ("pid", w.DWORD), ("tid", w.DWORD)]


class Accounting(c.Structure):
    _fields_ = [("times", c.c_longlong * 4), ("faults", w.DWORD), ("total", w.DWORD),
                ("active", w.DWORD), ("terminated", w.DWORD)]


create_process = api("CreateProcessW", w.BOOL, w.LPCWSTR, w.LPWSTR, c.c_void_p, c.c_void_p,
                     w.BOOL, w.DWORD, c.c_void_p, w.LPCWSTR,
                     c.POINTER(StartupInfoEx), c.POINTER(ProcessInfo))


def check(success):
    if not success:
        raise c.WinError(c.get_last_error())


def job_name(token):
    return "Global\\AmigaVerification-" + token


def drain_path(directory, token):
    return directory / f"job-{token}.drained"


def active_processes(handle):
    information = Accounting()
    check(query_job(handle, 1, c.byref(information), c.sizeof(information), None))
    return information.active


def job_process_ids(handle):
    capacity = 16
    while True:
        buffer = c.create_string_buffer(8 + c.sizeof(c.c_size_t) * capacity)
        if query_job(handle, 3, buffer, c.sizeof(buffer), None):
            count = w.DWORD.from_buffer(buffer, 4).value
            return list((c.c_size_t * count).from_buffer(buffer, 8))
        if c.get_last_error() != 234:  # ERROR_MORE_DATA
            check(False)
        capacity *= 2


def is_msvc_telemetry(image):
    parts = tuple(part.lower() for part in Path(image).parts)
    return (len(parts) >= 8 and parts[-8:-5] == ("vc", "tools", "msvc")
            and parts[-4] == "bin" and parts[-3].startswith("host")
            and parts[-1] == "vctip.exe")


def stop_msvc_telemetry(handle):
    """Stop only confirmed telemetry when no other job members remain."""
    with contextlib.ExitStack() as resources:
        telemetry = []
        for pid in job_process_ids(handle):
            # QUERY_LIMITED_INFORMATION | TERMINATE | SYNCHRONIZE. Holding the
            # original handle keeps PID reuse from targeting another process.
            process = open_process(0x1000 | 0x0001 | 0x100000, False, pid)
            if not process:
                if c.get_last_error() == 87:  # The process already exited.
                    continue
                return
            resources.callback(close_handle, process)
            member = w.BOOL()
            check(process_in_job(process, handle, c.byref(member)))
            if not member.value:
                continue
            image = c.create_unicode_buffer(32768)
            length = w.DWORD(len(image))
            if not process_image(process, 0, image, c.byref(length)) or not is_msvc_telemetry(image.value):
                return
            telemetry.append((pid, process))
        for pid, process in telemetry:
            if wait_handle(process, 0) == 0:
                continue
            if not terminate_process(process, 0) and wait_handle(process, 0) != 0:
                check(False)
            if wait_handle(process, 5000) != 0:
                raise RuntimeError("MSVC telemetry did not stop; verification remains active.")
            print(f"[INFO] Stopped verification-owned MSVC telemetry (PID {pid}).", flush=True)


def ensure_stopped(directory, token):
    handle = open_job(0x0004, False, job_name(token))  # JOB_OBJECT_QUERY
    if not handle:
        error = c.get_last_error()
        if error != 2:
            raise RuntimeError(f"Cannot query previous verification Job Object: {c.WinError(error)}")
        # A job's name can disappear on last-handle closure while its processes
        # survive. Absence alone never proves that its writers have stopped.
        marker = drain_path(directory, token)
        if not marker.is_file() or marker.read_text(encoding="ascii") != token:
            raise RuntimeError("Cannot establish previous batch ownership: Job Object is unavailable "
                               "and its guardian did not confirm that all commands stopped.")
        return
    try:
        stop_msvc_telemetry(handle)
        if active_processes(handle):
            raise RuntimeError("Previous verification command is still active in its Job Object; "
                               "retry after it stops.")
    except OSError as error:
        raise RuntimeError(f"Cannot query previous verification Job Object: {error}") from error
    finally:
        close_handle(handle)


class Process:
    def __init__(self, information):
        self.handle = information.process
        self.pid = information.pid
        close_handle(information.thread)

    def wait(self, timeout=None):
        status = wait_handle(self.handle, 0xffffffff if timeout is None else int(timeout * 1000))
        if status == 258:
            raise subprocess.TimeoutExpired(str(self.pid), timeout)
        check(status == 0)
        code = w.DWORD()
        check(exit_code(self.handle, c.byref(code)))
        return code.value

    def close(self):
        close_handle(self.handle)


def launch(command, cwd, env, output, job=None, inherited=(), creationflags=0):
    """Assign JOB_LIST inside CreateProcess, before any command code can run."""
    with contextlib.ExitStack() as resources:
        null = resources.enter_context(open(os.devnull, "rb"))
        stdin = get_std_handle(-10 & 0xffffffff)
        if stdin in (None, c.c_void_p(-1).value):
            stdin = msvcrt.get_osfhandle(null.fileno())
        handles = []
        for handle in (stdin, msvcrt.get_osfhandle(output.fileno())):
            duplicate = w.HANDLE()
            check(duplicate_handle(w.HANDLE(-1), handle, w.HANDLE(-1), c.byref(duplicate), 0, True, 2))
            resources.callback(close_handle, duplicate)
            handles.append(duplicate.value)
        inherited_handles = list(dict.fromkeys([*handles, *inherited]))
        attributes = [(0x20002, (w.HANDLE * len(inherited_handles))(*inherited_handles))]  # HANDLE_LIST
        if job is not None:
            attributes.append((0x2000d, (w.HANDLE * 1)(job)))  # JOB_LIST (Windows 10+)
        size = c.c_size_t()
        initialize_attributes(None, len(attributes), 0, c.byref(size))
        buffer = c.create_string_buffer(size.value)
        check(initialize_attributes(buffer, len(attributes), 0, c.byref(size)))
        resources.callback(delete_attributes, buffer)
        for key, value in attributes:
            check(update_attribute(buffer, 0, key, value, c.sizeof(value), None, None))
        startup = StartupInfoEx()
        startup.startup.cb = c.sizeof(startup)
        startup.startup.flags = 0x100  # STARTF_USESTDHANDLES
        startup.startup.stdin, startup.startup.stdout = handles
        startup.startup.stderr = handles[1]
        startup.attributes = c.cast(buffer, c.c_void_p)
        environment = c.create_unicode_buffer("\0".join(
            f"{key}={value}" for key, value in sorted(env.items(), key=lambda item: item[0].upper())) + "\0\0")
        information = ProcessInfo()
        check(create_process(None, c.create_unicode_buffer(subprocess.list2cmdline(command)),
                             None, None, True, 0x80000 | 0x400 | creationflags, environment, str(cwd),
                             c.byref(startup), c.byref(information)))
        return Process(information)


class WindowsJob:
    def __init__(self, directory, token):
        self.handle = create_job(None, job_name(token))
        check(self.handle)
        self.writer = None
        self.guardian = None
        try:
            if c.get_last_error() == 183:
                raise RuntimeError("Verification Job Object already exists; ownership is uncertain.")
            reader, self.writer = os.pipe()
            try:
                os.set_handle_inheritable(self.handle, True)
                os.set_inheritable(reader, True)
                reader_handle = msvcrt.get_osfhandle(reader)
                with open(os.devnull, "wb") as output:
                    self.guardian = launch(
                        [sys.executable, str(Path(__file__).resolve()), str(self.handle),
                         str(reader_handle), str(drain_path(directory, token)), token],
                        directory, os.environ, output, inherited=(self.handle, reader_handle),
                        creationflags=0x08000000)  # CREATE_NO_WINDOW: survive console interruption.
            finally:
                os.close(reader)
                os.set_handle_inheritable(self.handle, False)
        except BaseException:
            self.release()
            raise

    @contextlib.contextmanager
    def popen(self, command, cwd, env):
        reader, writer = os.pipe()
        with os.fdopen(reader, "r", encoding="utf-8", errors="replace") as stream:
            with os.fdopen(writer, "wb") as output:
                process = launch(command, cwd, env, output, job=self.handle)
            process.stdout = stream
            try:
                yield process
            finally:
                process.close()

    def complete(self):
        stop_msvc_telemetry(self.handle)
        if active_processes(self.handle):
            raise RuntimeError("Previous verification command is still active in its Job Object; "
                               "results cannot be completed until all descendants stop.")

    def release(self):
        if self.writer is not None:
            os.close(self.writer)
            self.writer = None
        try:
            if self.guardian is not None:
                # Never wait for or terminate orphan commands here. The guardian
                # remains alive independently until the whole job is empty.
                if not active_processes(self.handle):
                    self.guardian.wait(timeout=5)
        finally:
            if self.guardian is not None:
                self.guardian.close()
                self.guardian = None
            close_handle(self.handle)


def guard(handle, reader, marker, token):
    try:
        with os.fdopen(msvcrt.open_osfhandle(reader, os.O_RDONLY), "rb") as control:
            control.read()  # EOF means the owner exited or released the batch.
        while active_processes(handle):
            time.sleep(0.02)
        marker = Path(marker)
        pending = marker.with_suffix(".tmp")
        pending.write_text(token, encoding="ascii")
        pending.replace(marker)
    finally:
        close_handle(handle)


if __name__ == "__main__":
    guard(int(sys.argv[1]), int(sys.argv[2]), sys.argv[3], sys.argv[4])
