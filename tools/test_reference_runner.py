"""Exercise owned subprocess groups through the real reference command runner"""

from contextlib import redirect_stdout
import io
import json
import os
from pathlib import Path
import signal
import sys
import tempfile
import threading
import time
import unittest

from run_reference_oracles import run_command


@unittest.skipUnless(sys.platform == "linux", "Reference process-group fixtures require Linux")
class ReferenceRunnerTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="bgr-runner-test-")
        self.addCleanup(directory.cleanup)
        self.run = Path(directory.name)
        self.receipts = []

    def command(self, script, *arguments, timeout=3):
        with redirect_stdout(io.StringIO()):
            return run_command([sys.executable, "-c", script, *map(str, arguments)],
                               self.run, self.receipts, timeout=timeout)

    def assert_stopped(self, pid):
        deadline = time.monotonic() + 2
        while time.monotonic() < deadline:
            try:
                status = Path(f"/proc/{pid}/stat").read_text().rsplit(") ", 1)[1].split()[0]
            except (FileNotFoundError, ProcessLookupError):
                return
            if status in {"Z", "X"}:
                return
            time.sleep(0.01)
        self.fail(f"Owned child {pid} remains running")

    def assert_receipt(self):
        self.assertEqual(json.loads((self.run / "commands.json").read_text()), self.receipts)
        receipt = self.receipts[-1]
        self.assertEqual(receipt["process_group"], receipt["pid"])
        self.assertNotEqual(receipt["process_group"], os.getpgrp())
        self.assert_stopped(receipt["pid"])
        return receipt

    def test_success_preserves_output_and_records_owned_group(self):
        output = self.command('import sys;print("stdout");print("stderr",file=sys.stderr)')
        self.assertEqual(output, "stdout\n")
        receipt = self.assert_receipt()
        self.assertEqual(receipt["stderr"], "stderr\n")
        self.assertEqual(receipt["exit_code"], 0)
        self.assertFalse(receipt["interrupted"])
        self.assertFalse(receipt["timed_out"])

    def test_timeout_stops_child_and_preserves_partial_output(self):
        with self.assertRaises(ValueError):
            self.command('import time;print("before timeout",flush=True);time.sleep(120)', timeout=0.2)
        receipt = self.assert_receipt()
        self.assertEqual(receipt["stdout"], "before timeout\n")
        self.assertTrue(receipt["timed_out"])
        self.assertFalse(receipt["interrupted"])
        self.assertNotEqual(receipt["exit_code"], 0)

    def test_signal_cancellation_persists_logs_before_reraising(self):
        ready = self.run / "ready"
        stop = threading.Event()

        def cancel_when_ready():
            deadline = time.monotonic() + 2
            while not stop.is_set() and time.monotonic() < deadline:
                if ready.exists():
                    os.kill(os.getpid(), signal.SIGUSR1)
                    return
                stop.wait(0.01)

        def cancel(_signum, _frame):
            raise SystemExit(143)

        previous = signal.signal(signal.SIGUSR1, cancel)
        worker = threading.Thread(target=cancel_when_ready)
        worker.start()
        try:
            with self.assertRaises(SystemExit) as raised:
                self.command('import pathlib,sys,time;print("cancel stdout",flush=True);'
                             'print("cancel stderr",file=sys.stderr,flush=True);'
                             'pathlib.Path(sys.argv[1]).write_text("ready");time.sleep(120)', ready)
            self.assertEqual(raised.exception.code, 143)
        finally:
            stop.set()
            worker.join(timeout=3)
            signal.signal(signal.SIGUSR1, previous)
        self.assertFalse(worker.is_alive())
        receipt = self.assert_receipt()
        self.assertEqual(receipt["stdout"], "cancel stdout\n")
        self.assertEqual(receipt["stderr"], "cancel stderr\n")
        self.assertTrue(receipt["interrupted"])
        self.assertFalse(receipt["timed_out"])
        self.assertIn('"interrupted": true', (self.run / "progress.log").read_text())

    def test_exited_wrappers_leave_no_redirected_descendant(self):
        script = '''import json,os,pathlib,subprocess,sys,time
child_script = "import os,pathlib,signal,sys,time;signal.signal(signal.SIGTERM,signal.SIG_IGN);pathlib.Path(sys.argv[1]).write_text(str(os.getpid()));time.sleep(120)"
ready = pathlib.Path(sys.argv[1])
child = subprocess.Popen([sys.executable,"-c",child_script,str(ready)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
deadline = time.monotonic() + 2
while not ready.exists() and time.monotonic() < deadline: time.sleep(0.01)
if not ready.exists(): raise RuntimeError("Child fixture did not initialize")
print(json.dumps({"wrapper":os.getpid(),"child":child.pid,"group":os.getpgrp()}),flush=True)
sys.exit(int(sys.argv[2]))
'''
        for exit_code in [0, 1]:
            with self.subTest(exit_code=exit_code):
                ready = self.run / f"child-{exit_code}.pid"
                count = len(self.receipts)
                try:
                    if exit_code:
                        with self.assertRaises(ValueError):
                            self.command(script, ready, exit_code)
                    else:
                        self.command(script, ready, exit_code)
                    receipt = self.assert_receipt()
                    observed = json.loads(receipt["stdout"])
                    self.assertEqual(receipt["pid"], observed["wrapper"])
                    self.assertEqual(receipt["process_group"], observed["group"])
                    self.assertEqual(receipt["exit_code"], exit_code)
                    self.assert_stopped(observed["child"])
                finally:
                    if len(self.receipts) > count:
                        try:
                            os.killpg(self.receipts[-1]["process_group"], signal.SIGKILL)
                        except ProcessLookupError:
                            pass


if __name__ == "__main__":
    unittest.main()
