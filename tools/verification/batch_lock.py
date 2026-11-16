"""One process-held verification lock per repository working directory."""
import errno
import os


class VerificationLock:
    def __init__(self, root):
        self.path = root / ".agent/tmp/verification.lock"
        self.handle = None

    def __enter__(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.handle = self.path.open("a+b")
        try:
            os.set_inheritable(self.handle.fileno(), False)
            self.handle.seek(0)
            try:
                if os.name == "nt":
                    import msvcrt
                    msvcrt.locking(self.handle.fileno(), msvcrt.LK_NBLCK, 1)
                else:
                    import fcntl
                    fcntl.flock(self.handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except OSError as error:
                busy = (errno.EACCES,) if os.name == "nt" else (errno.EAGAIN, errno.EWOULDBLOCK)
                if error.errno in busy:
                    raise BlockingIOError(
                        "Verification is already running in this repository working directory."
                    ) from error
                raise
        except BaseException:
            self.handle.close()
            self.handle = None
            raise
        return self

    def __exit__(self, *exc):
        self.handle.close()
        self.handle = None
