"""Process ownership for resumable verification, including surviving commands."""
import json
import os
import time
import uuid
import re

import psutil


RUN_MARKERS = "AMIGA_VERIFICATION_RUNS"


def ensure_commands_stopped(record, directory):
    """Fail closed when a previous command or uncertain process ownership remains."""
    if os.name == "nt":
        from windows_job import ensure_stopped
        return ensure_stopped(directory, record["token"])
    # Preserve the existing conservative ownership check on POSIX. Environment
    # markers are not used for Windows command containment or recovery.
    if (not isinstance(record.get("started"), (int, float)) or record["started"] <= 0
            or not isinstance(record.get("user"), str) or not record["user"]):
        raise ValueError("Incomplete process ownership record")
    for _ in range(3):
        before = set(psutil.pids())
        for pid in before:
            try:
                process = psutil.Process(pid)
                if process.create_time() < record["started"]:
                    continue
                if process.username() != record["user"]:
                    continue
                markers = process.environ().get(RUN_MARKERS, "").split(",")
                if record["token"] in markers:
                    raise RuntimeError(f"Previous verification command is still active (PID {pid}); retry after it stops.")
            except (psutil.NoSuchProcess, psutil.ZombieProcess):
                continue
            except psutil.AccessDenied as error:
                raise RuntimeError(f"Cannot establish ownership of PID {pid}; retry after the previous run stops.") from error
        if before == set(psutil.pids()):
            return
    raise RuntimeError("Process ownership changed during recovery; repeat the same command.")


class BatchLock:
    def __init__(self, directory):
        self.path = directory / "active.lock"
        self.handle = None
        self.token = uuid.uuid4().hex
        self.job = None

    def acquire(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.handle = self.path.open("a+b")
        try:
            self.handle.seek(0)
            if os.name == "nt":
                import msvcrt
                msvcrt.locking(self.handle.fileno(), msvcrt.LK_NBLCK, 1)
            else:
                import fcntl
                fcntl.flock(self.handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError as error:
            self.handle.close()
            self.handle = None
            raise RuntimeError("Verification is active in this results directory; wait for its owner.") from error
        try:
            self.handle.seek(0)
            raw = self.handle.read()
            if raw:
                record = json.loads(raw)
                backend = "windows-job" if os.name == "nt" else "process-markers"
                if (not isinstance(record, dict) or record.get("version") != 2
                        or record.get("backend") != backend
                        or not isinstance(record.get("active"), bool)
                        or not isinstance(record.get("token"), str)
                        or not re.fullmatch("[0-9a-f]{32}", record["token"])):
                    raise ValueError("Incomplete process ownership record")
                if record["active"]:
                    ensure_commands_stopped(record, self.path.parent)
                    print("[INFO] Resuming an interrupted batch in the same results directory.")
            self.record = {"version": 2, "active": True, "token": self.token,
                           "backend": "windows-job" if os.name == "nt" else "process-markers"}
            if os.name != "nt":
                self.record.update(started=time.time(), user=psutil.Process().username())
            self.save()
            if os.name == "nt":
                from windows_job import WindowsJob
                self.job = WindowsJob(self.path.parent, self.token)
        except (ValueError, KeyError, TypeError, psutil.Error) as error:
            self.release()
            raise RuntimeError("Cannot establish previous batch ownership from active.lock.") from error
        except BaseException:
            self.release()
            raise

    def save(self):
        self.handle.seek(0)
        self.handle.truncate()
        self.handle.write(json.dumps(self.record).encode("utf-8"))
        self.handle.flush()

    def command_environment(self, overrides):
        if self.job is not None:
            return overrides
        # Keep ancestor markers when a contract test starts a nested verifier.
        ancestors = os.environ.get(RUN_MARKERS, "")
        return {**overrides, RUN_MARKERS: ",".join(filter(None, (ancestors, self.token)))}

    def complete(self):
        if self.job is not None:
            self.job.complete()
        self.record["active"] = False
        self.save()

    def release(self):
        try:
            if self.job is not None:
                self.job.release()
                self.job = None
        finally:
            if self.handle is not None:
                # Keep the file in place so concurrent openers lock the same file.
                self.handle.close()
                self.handle = None
