"""Exercise the real frontend with XTest input on a caller-owned Xvfb display"""

import argparse
import ctypes as c
import ctypes.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True
from build import ROOT, executable, godot_errors, local_path
from run_reference_oracles import run_command


class NativeInput:
    def __init__(self):
        def bind(library, name, result, arguments):
            function = getattr(library, name)
            function.restype, function.argtypes = result, arguments
            return function

        x11_name, test_name = ctypes.util.find_library("X11"), ctypes.util.find_library("Xtst")
        if not x11_name or not test_name:
            raise ValueError("Installed X11 and Xtst libraries are required")
        x11, test = c.CDLL(x11_name), c.CDLL(test_name)
        pointer, window = c.c_void_p, c.c_ulong
        bind(x11, "XOpenDisplay", pointer, [c.c_char_p])
        self.close = bind(x11, "XCloseDisplay", c.c_int, [pointer])
        self.display = x11.XOpenDisplay(None)
        if not self.display:
            raise ValueError("Cannot open the test display")
        self.flush = bind(x11, "XFlush", c.c_int, [pointer])
        self.root = bind(x11, "XDefaultRootWindow", window, [pointer])(self.display)
        self.query = bind(x11, "XQueryTree", c.c_int,
                          [pointer, window, c.POINTER(window), c.POINTER(window), c.POINTER(c.POINTER(window)), c.POINTER(c.c_uint)])
        self.fetch_name = bind(x11, "XFetchName", c.c_int, [pointer, window, c.POINTER(c.c_void_p)])
        self.free = bind(x11, "XFree", c.c_int, [pointer])
        self.focus = bind(x11, "XSetInputFocus", c.c_int, [pointer, window, c.c_int, c.c_ulong])
        self.resize = bind(x11, "XResizeWindow", c.c_int, [pointer, window, c.c_uint, c.c_uint])
        self.keysym = bind(x11, "XStringToKeysym", c.c_ulong, [c.c_char_p])
        self.keycode = bind(x11, "XKeysymToKeycode", c.c_uint8, [pointer, c.c_ulong])
        self.send_key = bind(test, "XTestFakeKeyEvent", c.c_int, [pointer, c.c_uint, c.c_int, c.c_ulong])
        self.motion = bind(test, "XTestFakeMotionEvent", c.c_int, [pointer, c.c_int, c.c_int, c.c_int, c.c_ulong])
        self.button = bind(test, "XTestFakeButtonEvent", c.c_int, [pointer, c.c_uint, c.c_int, c.c_ulong])
        query_test = bind(test, "XTestQueryExtension", c.c_int, [pointer, *([c.POINTER(c.c_int)] * 4)])
        fields = [c.c_int() for _ in range(4)]
        if not query_test(self.display, *(c.byref(field) for field in fields)):
            self.close(self.display)
            raise ValueError("XTest is unavailable on the test display")

    def find_window(self, title):
        queue = [self.root]
        observed_titles = []
        for _ in range(128):
            if not queue:
                break
            current = queue.pop(0)
            name = c.c_void_p()
            if self.fetch_name(self.display, current, c.byref(name)) and name.value:
                try:
                    observed = c.string_at(name).decode("utf-8", "replace")
                    observed_titles.append(observed)
                    if observed in {title, title + " (DEBUG)"}:
                        return current
                finally:
                    self.free(name)
            root, parent, count = c.c_ulong(), c.c_ulong(), c.c_uint()
            children = c.POINTER(c.c_ulong)()
            if self.query(self.display, current, c.byref(root), c.byref(parent), c.byref(children), c.byref(count)):
                try:
                    queue.extend(children[index] for index in range(count.value))
                finally:
                    if children:
                        self.free(children)
        raise ValueError("Biogenesis-R window is absent from the private display: " + repr(observed_titles))

    def key(self, name):
        self.set_key(name, True)
        self.set_key(name, False)
        self.flush(self.display)

    def set_key(self, name, pressed):
        code = self.keycode(self.display, self.keysym(name.encode("ascii")))
        if not code:
            raise ValueError("Unmapped test key: " + name)
        self.send_key(self.display, code, int(pressed), 0)

    def point_at(self, observation, control):
        x, y, width, height = observation["controls"][control]
        px, py = observation["window_position"]
        self.motion(self.display, -1, round(px + x + width / 2), round(py + y + height / 2), 0)

    def click(self, observation, control):
        self.point_at(observation, control)
        self.button(self.display, 1, 1, 0)
        self.button(self.display, 1, 0, 0)
        self.flush(self.display)

    def replace_text(self, observation, control, text):
        self.click(observation, control)
        self.set_key("Control_L", True)
        self.key("a")
        self.set_key("Control_L", False)
        for character in text:
            self.key(character)
        self.flush(self.display)


def inside(engine, run):
    if os.environ.get("BGR_TEST_ROOT") != str(run) or not os.environ.get("DISPLAY", "").startswith(":"):
        raise ValueError("Run this check through its owned Xvfb wrapper")
    log = run / "engine.log"
    events, decisions = [], []
    with log.open("wb") as output:
        process = subprocess.Popen([str(engine), "--audio-driver", "Dummy", "--path", str(ROOT), "--script", "res://ui/gui_check.gd",
                                    "--resolution", "1200x900"], stdout=output, stderr=subprocess.STDOUT)
        native = None
        try:
            def wait(marker):
                deadline = time.monotonic() + 15
                while time.monotonic() < deadline:
                    content = log.read_text(errors="replace")
                    if "BGR_GUI_FAIL" in content or godot_errors(content):
                        raise ValueError("The frontend reported an error; inspect " + str(log))
                    for line in content.splitlines():
                        if line.startswith(marker + " "):
                            observation = json.loads(line[len(marker) + 1:])
                            if marker == "BGR_GUI_EVENT" and observation.get("stage") != expected_stage:
                                continue
                            events.append(observation)
                            return observation
                    if process.poll() is not None:
                        raise ValueError("The frontend exited before " + marker)
                    time.sleep(0.025)
                raise ValueError("The frontend did not report " + marker)

            def confirm_world_decision(operation, offset):
                deadline = time.monotonic() + 15
                while time.monotonic() < deadline:
                    with log.open("r", errors="replace") as stream:
                        stream.seek(offset)
                        content = stream.read()
                    if "BGR_GUI_FAIL" in content or godot_errors(content):
                        raise ValueError("Frontend error before " + operation + " confirmation")
                    for line in content.splitlines(keepends=True):
                        prefix = "BGR_GUI_DECISION "
                        if line.endswith("\n") and line.startswith(prefix):
                            observed = json.loads(line[len(prefix):])
                            if observed["decision_visible"] and observed["decision_controls_ready"] and observed["decision"] == operation:
                                native.click(observed, "ConfirmWorldDecision")
                                decisions.append({"operation": operation, "observation": observed})
                                return
                    if process.poll() is not None:
                        raise ValueError("Frontend exited before " + operation + " confirmation")
                    time.sleep(0.025)
                raise ValueError("Frontend did not expose " + operation + " confirmation")

            expected_stage = ""
            ready = wait("BGR_GUI_READY")
            native = NativeInput()
            window = native.find_window("Biogenesis-R")
            native.focus(native.display, window, 2, 0)
            native.flush(native.display)
            for expected_stage, key in [("STEP", "n"), ("RUNNING", "space"), ("PAUSED", "space"), ("RESET", "r")]:
                request_start = log.stat().st_size
                native.key(key)
                if key == "r":
                    confirm_world_decision("reset", request_start)
                state = wait("BGR_GUI_EVENT")
            expected_stage = "ZOOM"
            native.point_at(state, "WorldCanvas")
            native.button(native.display, 4, 1, 0)
            native.button(native.display, 4, 0, 0)
            native.flush(native.display)
            state = wait("BGR_GUI_EVENT")
            expected_stage = "FIT"
            native.key("f")
            state = wait("BGR_GUI_EVENT")
            expected_stage = "RESIZED"
            native.resize(native.display, window, 900, 720)
            native.flush(native.display)
            state = wait("BGR_GUI_EVENT")
            expected_stage = "PAN"
            native.point_at(state, "WorldCanvas")
            native.button(native.display, 2, 1, 0)
            native.flush(native.display)
            time.sleep(0.05)
            x, y, width, height = state["controls"]["WorldCanvas"]
            px, py = state["window_position"]
            native.motion(native.display, -1, round(px + x + width / 2 + 40), round(py + y + height / 2 + 20), 0)
            native.button(native.display, 2, 0, 0)
            native.flush(native.display)
            state = wait("BGR_GUI_EVENT")
            expected_stage = "MINIMUM"
            native.resize(native.display, window, 800, 660)
            native.flush(native.display)
            state = wait("BGR_GUI_EVENT")
            expected_stage = "CONFIG_ERROR"
            native.replace_text(state, "WidthInput", "0")
            request_start = log.stat().st_size
            native.click(state, "CreateWorldButton")
            confirm_world_decision("create", request_start)
            state = wait("BGR_GUI_EVENT")
            expected_stage = "NEW_WORLD"
            native.replace_text(state, "WidthInput", "1200")
            request_start = log.stat().st_size
            native.click(state, "CreateWorldButton")
            confirm_world_decision("create", request_start)
            state = wait("BGR_GUI_EVENT")
            for expected_stage, control in [("BUTTON_STEP", "StepButton"), ("BUTTON_RUNNING", "PauseButton"),
                                             ("BUTTON_PAUSED", "PauseButton"), ("BUTTON_RESET", "ResetButton")]:
                request_start = log.stat().st_size
                native.click(state, control)
                if control == "ResetButton":
                    confirm_world_decision("reset", request_start)
                state = wait("BGR_GUI_EVENT")
            time.sleep(0.15)
            request_start = log.stat().st_size
            native.key("q")
            confirm_world_decision("quit", request_start)
            status = process.wait(timeout=5)
            if status:
                raise ValueError("The frontend returned " + str(status))
            if godot_errors(log.read_text(errors="replace")):
                raise ValueError("The frontend reported a late error; inspect " + str(log))
            result = {"result": "PASS", "exit_code": status, "events": events, "world_decisions": decisions,
                      "input": "native XTest", "display": "owned Xvfb", "initial": ready}
            (run / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
            print(json.dumps({"result": "PASS", "stages": [event.get("stage", "READY") for event in events]}), flush=True)
        finally:
            if native:
                native.close(native.display)
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=2)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--godot")
    parser.add_argument("--inside", type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args()
    engine = executable(args.godot, "godot")
    if args.inside:
        inside(engine, local_path(ROOT, args.inside))
        return
    xvfb = executable(None, "xvfb-run")
    runs = local_path(ROOT, ROOT / ".workflow-local/native-controls")
    runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix="journey-", dir=runs))
    env = {key: os.environ[key] for key in ["LANG", "LC_ALL", "TZ"] if key in os.environ}
    for key, name in [("HOME", "home"), ("XDG_CONFIG_HOME", "config"), ("XDG_CACHE_HOME", "cache"),
                      ("XDG_DATA_HOME", "data"), ("TMPDIR", "tmp")]:
        path = run / name
        path.mkdir()
        env[key] = str(path)
    env.update(PATH=os.defpath, PYTHONDONTWRITEBYTECODE="1", BGR_TEST_ROOT=str(run), BGR_GUI_OUTPUT=str(run))
    print(json.dumps({"run_root": str(run)}), flush=True)
    print(run_command([str(xvfb), "-a", "-s", "-screen 0 1280x960x24", sys.executable, str(Path(__file__).resolve()),
                       "--godot", str(engine), "--inside", str(run)], run, [], env=env, timeout=75), flush=True)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
