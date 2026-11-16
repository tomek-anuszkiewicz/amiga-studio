"""Real Windows containment, nested verifier and uncertain-ownership contracts."""
import ctypes
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch
import psutil

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from batch_lock import BatchLock

if os.name == "nt":
    import windows_job


@unittest.skipUnless(os.name == "nt", "Windows Job Objects")
class WindowsJobTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def wait_for(self, path):
        deadline = time.monotonic() + 10
        while not path.exists() and time.monotonic() < deadline:
            time.sleep(0.01)
        self.assertTrue(path.exists(), f"Fixture did not produce {path.name}")

    def prelude(self):
        return (f"import sys; sys.path.insert(0, {str(Path(windows_job.__file__).parent)!r}); "
                "from pathlib import Path; import subprocess,time; from batch_lock import BatchLock; ")

    def test_native_launch_preserves_arguments_environment_output_and_exit_code(self):
        lock = BatchLock(self.root)
        lock.acquire()
        directory = self.root / "working directory"
        directory.mkdir()
        arguments = ["with spaces", 'a"quote', "trailing\\", "\u03bb"]
        code = ("import json,os,sys; "
                "print(json.dumps([sys.argv[1:],os.environ['JOB_TEST'],os.getcwd()]),flush=True); "
                "print('stderr output',file=sys.stderr,flush=True); sys.exit(7)")
        try:
            with lock.job.popen([sys.executable, "-c", code, *arguments], cwd=directory,
                                env={**os.environ, "JOB_TEST": "value with spaces \u03bb"}) as child:
                lines = child.stdout.read().splitlines()
                self.assertEqual(child.wait(timeout=5), 7)
            self.assertEqual(json.loads(lines[0]), [arguments, "value with spaces \u03bb", str(directory)])
            self.assertEqual(lines[1:], ["stderr output"])
            lock.complete()
        finally:
            lock.release()

    def test_nested_jobs_retain_orphan_after_inner_verifier_exits(self):
        outer = BatchLock(self.root / "outer")
        outer.acquire()
        inner_directory = self.root / "inner"
        ready, release = self.root / "ready", self.root / "release"
        # The first command instruction checks membership in both jobs. It then
        # outlives its verifier without inheriting any job or lock-file handle.
        leaf = (self.prelude() + "import windows_job as j; import ctypes as c; "
                "inside=j.api('IsProcessInJob', j.w.BOOL, j.w.HANDLE, j.w.HANDLE, c.POINTER(j.w.BOOL)); "
                f"tokens=[{outer.token!r}, sys.argv[1]]; "
                "\nfor token in tokens:\n h=j.open_job(4,False,j.job_name(token)); j.check(h); "
                "ok=j.w.BOOL(); j.check(inside(j.w.HANDLE(-1),h,c.byref(ok))); assert ok.value; j.close_handle(h)\n"
                f"Path({str(ready)!r}).write_text(str(__import__('os').getpid()))\n"
                f"while not Path({str(release)!r}).exists(): time.sleep(.01)\n")
        inner = (self.prelude() + f"lock=BatchLock(Path({str(inner_directory)!r})); lock.acquire(); "
                 f"\nwith lock.job.popen({[sys.executable, '-c', leaf]!r} + [lock.token], cwd=Path({str(self.root)!r}), env=__import__('os').environ) as child:\n print(child.stdout.read(), flush=True); child.wait()")
        try:
            with outer.job.popen([sys.executable, "-c", inner], cwd=self.root, env=os.environ) as verifier:
                try:
                    self.wait_for(ready)
                except AssertionError:
                    try:
                        verifier.wait(timeout=0)
                    except subprocess.TimeoutExpired:
                        raise
                    self.fail(verifier.stdout.read())
                finally:
                    # Terminate only this fixture's verifier; the leaf must finish itself.
                    if psutil.pid_exists(verifier.pid):
                        psutil.Process(verifier.pid).kill()
                    verifier.wait(timeout=5)
            inner_record = json.loads((inner_directory / "active.lock").read_text())
            outer.release()
            for directory in (outer.path.parent, inner_directory):
                with self.assertRaisesRegex(RuntimeError, "Previous verification command is still active"):
                    BatchLock(directory).acquire()
            release.touch()
            for directory, token in ((outer.path.parent, outer.token), (inner_directory, inner_record["token"])):
                self.wait_for(windows_job.drain_path(directory, token))
                recovered = BatchLock(directory)
                recovered.acquire()
                recovered.complete()
                recovered.release()
        finally:
            release.touch()
            if outer.handle is not None:
                outer.release()
            if ready.exists() and ready.read_text():
                try:
                    psutil.Process(int(ready.read_text())).wait(timeout=10)
                except psutil.NoSuchProcess:
                    pass

    def test_missing_job_requires_intact_guardian_confirmation(self):
        lock = BatchLock(self.root)
        lock.acquire()
        record = dict(lock.record)
        lock.release()
        marker = windows_job.drain_path(self.root, lock.token)
        self.wait_for(marker)
        self.assertEqual(marker.read_text(), lock.token)
        for contents in (None, "wrong-token", "", "\ufffd"):
            if contents is None:
                marker.unlink(missing_ok=True)
            else:
                marker.write_text(contents, encoding="utf-8")
            with self.subTest(contents=contents), self.assertRaises(RuntimeError):
                BatchLock(self.root).acquire()
            self.assertEqual(json.loads(lock.path.read_text()), record)
        marker.write_text(lock.token, encoding="ascii")
        recovered = BatchLock(self.root)
        recovered.acquire()
        recovered.complete()
        recovered.release()

    def test_inaccessible_or_unqueryable_job_preserves_metadata(self):
        lock = BatchLock(self.root)
        lock.acquire()
        lock.release()
        previous = lock.path.read_bytes()
        with patch.object(windows_job, "open_job", return_value=None):
            ctypes.set_last_error(5)
            with self.assertRaisesRegex(RuntimeError, "Cannot query previous verification Job Object"):
                BatchLock(self.root).acquire()
        with patch.object(windows_job, "active_processes", side_effect=OSError("query denied")):
            with self.assertRaisesRegex(RuntimeError, "Cannot query previous verification Job Object"):
                # Keep a real job handle alive so the open succeeds.
                handle = windows_job.create_job(None, windows_job.job_name(lock.token))
                try:
                    BatchLock(self.root).acquire()
                finally:
                    windows_job.close_handle(handle)
        self.assertEqual(lock.path.read_bytes(), previous)

    def detached_command(self, lock, ready, release):
        # Existence is the handshake: publish only after the complete PID is closed.
        leaf = ("from pathlib import Path; import time,os; "
                f"ready = Path({str(ready)!r}); pending = ready.with_name(ready.name + '.pending'); "
                "pending.write_text(str(os.getpid())); pending.replace(ready)\n"
                f"while not Path({str(release)!r}).exists(): time.sleep(.01)\n")
        parent = ("import subprocess,sys; "
                  f"subprocess.Popen([sys.executable,'-c',{leaf!r}], "
                  "stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)")
        with lock.job.popen([sys.executable, "-c", parent], cwd=self.root, env=os.environ) as process:
            self.assertEqual(process.wait(timeout=5), 0)
        self.wait_for(ready)
        return psutil.Process(int(ready.read_text()))

    def test_telemetry_identity_requires_the_msvc_toolchain_path(self):
        self.assertTrue(windows_job.is_msvc_telemetry(
            r"C:\Program Files\Microsoft Visual Studio\VC\Tools\MSVC\14.44\bin\HostX64\x64\VCTIP.EXE"))
        for image in (r"C:\unrelated\vctip.exe",
                      r"C:\VC\Tools\MSVC\14.44\bin\Hostx64\x64\rustc.exe"):
            with self.subTest(image=image):
                self.assertFalse(windows_job.is_msvc_telemetry(image))

    def test_completion_stops_confirmed_telemetry_only_inside_its_own_job(self):
        lock = BatchLock(self.root)
        lock.acquire()
        ready, release = self.root / "ready", self.root / "release"
        survivor = None
        unrelated = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(60)"],
                                     stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            survivor = self.detached_command(lock, ready, release)
            # Identify this controlled Python fixture as telemetry; the separate
            # identity test retains the actual MSVC path contract.
            with patch.object(windows_job, "is_msvc_telemetry", return_value=True):
                lock.complete()
            survivor.wait(timeout=5)
            self.assertIsNone(unrelated.poll(), "A process outside the job must survive")
            self.assertFalse(lock.record["active"])
        finally:
            release.touch()
            lock.release()
            unrelated.terminate()
            unrelated.wait(timeout=5)
            if survivor is not None and survivor.is_running():
                survivor.wait(timeout=5)

    def test_live_command_prevents_telemetry_cleanup(self):
        lock = BatchLock(self.root)
        lock.acquire()
        release = self.root / "release"
        survivors = []
        try:
            for number in range(2):
                survivors.append(self.detached_command(lock, self.root / f"ready-{number}", release))
            with patch.object(windows_job, "is_msvc_telemetry", side_effect=[True, False]):
                with self.assertRaisesRegex(RuntimeError, "all descendants stop"):
                    lock.complete()
            self.assertTrue(all(process.is_running() for process in survivors))
        finally:
            release.touch()
            for process in survivors:
                process.wait(timeout=5)
            lock.release()

    def test_successful_parent_cannot_complete_while_detached_child_writes(self):
        lock = BatchLock(self.root)
        lock.acquire()
        ready, release = self.root / "ready", self.root / "release"
        survivor = None
        try:
            survivor = self.detached_command(lock, ready, release)
            with self.assertRaisesRegex(RuntimeError, "all descendants stop"):
                lock.complete()
            self.assertTrue(lock.record["active"])
            lock.release()
            with self.assertRaisesRegex(RuntimeError, "Previous verification command is still active"):
                BatchLock(self.root).acquire()
        finally:
            release.touch()
            if lock.handle is not None:
                lock.release()
            if survivor is not None:
                survivor.wait(timeout=10)
        self.wait_for(windows_job.drain_path(self.root, lock.token))
        recovered = BatchLock(self.root)
        recovered.acquire()
        recovered.complete()
        recovered.release()

    def test_guardian_loss_cannot_make_live_orphan_look_finished(self):
        lock = BatchLock(self.root)
        lock.acquire()
        ready, release = self.root / "ready", self.root / "release"
        survivor = None
        try:
            survivor = self.detached_command(lock, ready, release)
            guardian = lock.job.guardian
            psutil.Process(guardian.pid).kill()
            guardian.wait(timeout=5)
            lock.release()
            with self.assertRaisesRegex(RuntimeError, "guardian did not confirm"):
                BatchLock(self.root).acquire()
            self.assertTrue(json.loads(lock.path.read_text())["active"])
        finally:
            release.touch()
            if lock.handle is not None:
                lock.release()
            if survivor is not None:
                survivor.wait(timeout=10)


if __name__ == "__main__":
    unittest.main()
