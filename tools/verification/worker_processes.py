"""Own test worker processes and the subprocesses they start."""

import os
from pathlib import Path
import select
import signal
import subprocess
import sys
import time


TERMINATION_GRACE_SECONDS = 0.5
CLEANUP_POLL_SECONDS = 0.02
JOB_CLEANUP_SECONDS = 5
PR_SET_CHILD_SUBREAPER = 36
START_WORKER = b"\n"


class WindowsJob:
    KILL_ON_JOB_CLOSE = 0x2000
    BASIC_ACCOUNTING_INFORMATION = 1
    EXTENDED_LIMIT_INFORMATION = 9
    PROCESS_SET_QUOTA = 0x0100
    PROCESS_TERMINATE = 0x0001

    def __init__(self):
        import ctypes
        from ctypes import wintypes

        class BasicLimits(ctypes.Structure):
            _fields_ = [
                ("per_process_time", ctypes.c_int64), ("per_job_time", ctypes.c_int64),
                ("flags", wintypes.DWORD), ("minimum_working_set", ctypes.c_size_t),
                ("maximum_working_set", ctypes.c_size_t), ("active_processes", wintypes.DWORD),
                ("affinity", ctypes.c_size_t), ("priority", wintypes.DWORD),
                ("scheduling_class", wintypes.DWORD),
            ]

        class ExtendedLimits(ctypes.Structure):
            _fields_ = [
                ("basic", BasicLimits), ("io_counters", ctypes.c_uint64 * 6),
                ("process_memory", ctypes.c_size_t), ("job_memory", ctypes.c_size_t),
                ("peak_process_memory", ctypes.c_size_t), ("peak_job_memory", ctypes.c_size_t),
            ]

        class BasicAccounting(ctypes.Structure):
            _fields_ = [
                ("user_time", ctypes.c_int64), ("kernel_time", ctypes.c_int64),
                ("period_user_time", ctypes.c_int64), ("period_kernel_time", ctypes.c_int64),
                ("page_faults", wintypes.DWORD), ("total_processes", wintypes.DWORD),
                ("active_processes", wintypes.DWORD), ("terminated_processes", wintypes.DWORD),
            ]

        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel.CreateJobObjectW.argtypes = [ctypes.c_void_p, wintypes.LPCWSTR]
        kernel.CreateJobObjectW.restype = wintypes.HANDLE
        kernel.SetInformationJobObject.argtypes = [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD]
        kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        kernel.OpenProcess.restype = wintypes.HANDLE
        kernel.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
        kernel.TerminateJobObject.argtypes = [wintypes.HANDLE, wintypes.UINT]
        kernel.QueryInformationJobObject.argtypes = [
            wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD, ctypes.c_void_p,
        ]
        kernel.CloseHandle.argtypes = [wintypes.HANDLE]

        self.kernel = kernel
        self.error = ctypes.WinError
        self.accounting = BasicAccounting()
        self.accounting_pointer = ctypes.byref(self.accounting)
        self.accounting_size = ctypes.sizeof(self.accounting)
        self.handle = kernel.CreateJobObjectW(None, None)
        if not self.handle:
            raise self.error()
        limits = ExtendedLimits()
        limits.basic.flags = self.KILL_ON_JOB_CLOSE
        if not kernel.SetInformationJobObject(self.handle, self.EXTENDED_LIMIT_INFORMATION,
                                              ctypes.byref(limits), ctypes.sizeof(limits)):
            error = self.error()
            self.close()
            raise error

    def attach(self, pid):
        process = self.kernel.OpenProcess(self.PROCESS_SET_QUOTA | self.PROCESS_TERMINATE, False, pid)
        if not process:
            raise self.error()
        try:
            if not self.kernel.AssignProcessToJobObject(self.handle, process):
                raise self.error()
        finally:
            self.kernel.CloseHandle(process)

    def close(self):
        self.kernel.CloseHandle(self.handle)

    def stop(self):
        if not self.kernel.TerminateJobObject(self.handle, 1):
            raise self.error()

    def wait(self):
        deadline = time.monotonic() + JOB_CLEANUP_SECONDS
        while True:
            if not self.kernel.QueryInformationJobObject(
                    self.handle, self.BASIC_ACCOUNTING_INFORMATION,
                    self.accounting_pointer, self.accounting_size, None):
                raise self.error()
            if not self.accounting.active_processes:
                return
            if time.monotonic() >= deadline:
                raise OSError("Test worker processes did not exit after job termination.")
            time.sleep(CLEANUP_POLL_SECONDS)


class WorkerProcesses:
    def __enter__(self):
        self.children = {}
        self.supervised = sys.platform == "linux"
        return self

    def start(self, command, log, environment, cwd):
        if self.supervised:
            command = [sys.executable, str(Path(__file__).resolve()), *command]
        job = WindowsJob() if os.name == "nt" else None
        try:
            process = subprocess.Popen(command, env=environment, cwd=cwd, stdin=subprocess.PIPE, stdout=log,
                                       stderr=subprocess.STDOUT, start_new_session=job is None)
        except BaseException:
            if job is not None:
                job.close()
            raise
        self.children[process] = job
        # Release startup only after the owner has recorded the process.
        if self.supervised:
            process.stdin.write(START_WORKER)
            process.stdin.flush()
        else:
            with process.stdin:
                if job is not None:
                    job.attach(process.pid)
                process.stdin.write(START_WORKER)
        return process

    def stop(self, process, job):
        if self.supervised:
            # Closing the control pipe tells the supervisor to stop and reap its tree.
            process.stdin.close()
        elif job is not None:
            job.stop()
        else:
            signal_group(process.pid, signal.SIGTERM)

    def wait(self, process, job):
        if job is not None:
            try:
                job.wait()
            finally:
                job.close()
        elif not self.supervised:
            deadline = time.monotonic() + TERMINATION_GRACE_SECONDS
            while signal_group(process.pid, 0) and time.monotonic() < deadline:
                process.poll()
                time.sleep(CLEANUP_POLL_SECONDS)
            signal_group(process.pid, signal.SIGKILL)
        process.wait()

    def finish(self, process):
        job = self.children[process]
        self.stop(process, job)
        del self.children[process]
        self.wait(process, job)

    def __exit__(self, *exc):
        errors = []
        for process, job in self.children.items():
            try:
                self.stop(process, job)
            except OSError as error:
                errors.append(error)
        for process, job in self.children.items():
            try:
                self.wait(process, job)
            except OSError as error:
                errors.append(error)
        if errors:
            raise errors[0]


def signal_group(group, sig):
    try:
        os.killpg(group, sig)
    except ProcessLookupError:
        return False
    return True


def supervise_linux_worker(command):
    import ctypes

    if not os.read(sys.stdin.fileno(), 1):
        return 1
    libc = ctypes.CDLL(None, use_errno=True)
    if libc.prctl(PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0):
        error = ctypes.get_errno()
        raise OSError(error, os.strerror(error))

    children = Path("/proc/thread-self/children")
    children.read_text(encoding="ascii")  # Check access before launching the test.
    with subprocess.Popen(command, stdin=subprocess.PIPE, start_new_session=True) as worker:
        try:
            with worker.stdin:
                worker.stdin.write(START_WORKER)
            while worker.poll() is None:
                if select.select([sys.stdin], [], [], CLEANUP_POLL_SECONDS)[0]:
                    worker.terminate()
                    try:
                        worker.wait(timeout=TERMINATION_GRACE_SECONDS)
                    except subprocess.TimeoutExpired:
                        worker.kill()
                    break
        finally:
            if worker.poll() is None:
                worker.kill()
            worker.wait()
            reap_linux_descendants(children)
    # Keep explicit failure statuses and use the shell convention for fatal signals.
    return worker.returncode if worker.returncode >= 0 else 128 - worker.returncode


def reap_linux_descendants(children):
    # Orphans are reparented here, even when they created another session.
    while True:
        try:
            while os.waitpid(-1, os.WNOHANG)[0]:
                pass
        except ChildProcessError:
            return
        for pid in children.read_text(encoding="ascii").split():
            # This single thread has not reaped these children, so their PIDs cannot be reused.
            try:
                os.kill(int(pid), signal.SIGKILL)
            except ProcessLookupError:
                pass
        time.sleep(CLEANUP_POLL_SECONDS)


if __name__ == "__main__":
    raise SystemExit(supervise_linux_worker(sys.argv[1:]))
