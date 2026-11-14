"""Process ownership for resumable verification, including surviving commands."""
import json
import os
import time
import uuid

import psutil


RUN_MARKERS = "AMIGA_VERIFICATION_RUNS"


def ensure_commands_stopped(record):
    """Fail closed when a previous command or uncertain process ownership remains."""
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
                if (not isinstance(record, dict) or not isinstance(record.get("active"), bool)
                        or not isinstance(record.get("token"), str) or len(record["token"]) != 32
                        or not isinstance(record.get("started"), (int, float)) or record["started"] <= 0
                        or not isinstance(record.get("user"), str) or not record["user"]):
                    raise ValueError("Incomplete process ownership record")
                if record["active"]:
                    ensure_commands_stopped(record)
                    print("[INFO] Resuming an interrupted batch in the same results directory.")
            self.record = {"active": True, "token": self.token, "started": time.time(),
                           "user": psutil.Process().username()}
            self.save()
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
        # Keep ancestor markers when a contract test starts a nested verifier.
        ancestors = os.environ.get(RUN_MARKERS, "")
        return {**overrides, RUN_MARKERS: ",".join(filter(None, (ancestors, self.token)))}

    def complete(self):
        self.record["active"] = False
        self.save()

    def release(self):
        if self.handle is not None:
            # Closing releases the kernel lock even after an exception. Keep the
            # file in place so concurrent openers always lock the same file.
            self.handle.close()
            self.handle = None
