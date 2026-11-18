"""Exercise laboratory Enter and ARGB controls through real XTest input"""

import hashlib
import json
import os
import signal
import subprocess
import time

from build import ROOT, godot_errors
from check_native_controls import NativeInput


def laboratory_extra_inside(engine, run):
    def require(value, message):
        if not value:
            raise ValueError(message)

    def save(path, value):
        path.write_text(json.dumps(value, indent=2) + "\n")

    def stop(number, _frame):
        raise SystemExit(128 + number)

    previous = {
        number: signal.signal(number, stop)
        for number in (signal.SIGTERM, signal.SIGINT)
    }
    native = None
    process = None
    actions = []
    checks = []
    phase = "startup"
    log = run / "engine.log"
    try:
        require(
            os.environ.get("BGR_TEST_ROOT") == str(run)
            and os.environ.get("DISPLAY", "").startswith(":"),
            "Owned display and run directory are required",
        )
        require(run.is_dir() and not log.exists(), "Preserve the existing native attempt")
        with log.open("xb") as output:
            process = subprocess.Popen(
                [
                    str(engine), "--audio-driver", "Dummy", "--path", str(ROOT),
                    "--script", "res://ui/laboratory_extra_controls_check.gd",
                    "--resolution", "1200x900",
                ],
                stdout=output,
                stderr=subprocess.STDOUT,
            )

        offset = 0
        partial = b""
        latest_row = None
        checkpoint_rows = []
        started = time.monotonic()

        def read():
            nonlocal offset, partial, latest_row
            with log.open("rb") as source:
                source.seek(offset)
                pending = partial + source.read()
                offset = source.tell()
            complete, _, partial = pending.rpartition(b"\n")
            text = complete.decode("utf-8", errors="replace")
            require(
                "BGR_GUI_FAIL" not in text and not godot_errors(text),
                "Frontend error; inspect engine.log",
            )
            for line in text.splitlines():
                if line.startswith("BGR_CRISP_OBS "):
                    latest_row = json.loads(line[len("BGR_CRISP_OBS "):])
                elif line.startswith("BGR_CRISP_CHECKPOINT "):
                    checkpoint_rows.append(json.loads(line[len("BGR_CRISP_CHECKPOINT "):]))
            return latest_row

        def wait(description, predicate, timeout=12):
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                require(time.monotonic() - started < 180, "Laboratory extra journey exceeded 180 seconds")
                row = read()
                if row and predicate(row):
                    return row
                require(process.poll() is None, "Frontend exited before " + description)
                time.sleep(0.015)
            row = read()
            detail = None if row is None else {
                key: row.get(key) for key in
                ("tick", "paused", "population", "laboratories", "application_error")
            }
            raise ValueError("Timed out " + description + "; observed=" + repr(detail))

        def key(name):
            native.key(name)
            actions.append({"key": name, "monotonic_ns": time.monotonic_ns()})

        def chord(modifier, name):
            native.set_key(modifier, True)
            try:
                key(name)
            finally:
                native.set_key(modifier, False)
                native.flush(native.display)

        def click(name):
            for _ in range(70):
                row = read()
                detail = row["control_details"].get(name, {})
                if row["control_ready"].get(name, False):
                    native.click(row, name)
                    actions.append({"control": name, "monotonic_ns": time.monotonic_ns()})
                    return
                if detail and not detail["inside"]:
                    require(detail["scroll"], "Control is outside its window: " + name)
                    rect = row["controls"][name]
                    clip = detail["clip"]
                    direction = 5 if rect[1] + rect[3] > clip[1] + clip[3] else 4
                    native.point_at(row, detail["scroll"])
                    native.button(native.display, direction, 1, 0)
                    native.button(native.display, direction, 0, 0)
                    native.flush(native.display)
                require(process.poll() is None, "Frontend exited before click " + name)
                time.sleep(0.025)
            raise ValueError("Actual control did not become clickable: " + name)

        def text(value):
            symbols = {"/": "slash", ".": "period", "-": "minus", " ": "space"}
            shifted = {"#": "3", ":": "semicolon", "_": "minus", "+": "equal"}
            for character in str(value):
                if character.isupper() or character in shifted:
                    native.set_key("Shift_L", True)
                    try:
                        native.key(character.lower() if character.isupper() else shifted[character])
                    finally:
                        native.set_key("Shift_L", False)
                else:
                    native.key(symbols.get(character, character))
            native.flush(native.display)

        def enter(name, value):
            click(name)
            chord("Control_L", "a")
            text(value)
            wait("actual entered text " + name, lambda row: row["texts"].get(name) == str(value))
            key("Return")
            actions.append({"entry": name, "value": str(value), "monotonic_ns": time.monotonic_ns()})

        def checkpoint(label):
            pending = run / "checkpoint-request.pending"
            save(pending, {"label": label})
            pending.replace(run / "checkpoint-request.json")
            wait("checkpoint " + label, lambda _row: any(row["label"] == label for row in checkpoint_rows))
            row = next(row for row in checkpoint_rows if row["label"] == label)
            checks.append(row)
            return row

        def menu(item):
            native.focus(native.display, window, 2, 0)
            native.flush(native.display)
            click("GeneticsMenu")
            wait("actual genetics popup", lambda row: row["genetics_menu"])
            for _ in range(16):
                focused = read()["genetics_focused_id"]
                if focused == item:
                    break
                key("Down")
                wait("actual genetics focus changes", lambda row: row["genetics_focused_id"] != focused)
            require(read()["genetics_focused_id"] == item, "Requested genetics item has no actual focus")
            key("Return")
            wait("actual genetics popup closes", lambda row: not row["genetics_menu"])

        def open_lab(item):
            require(not read()["laboratories"], "This focused journey uses one laboratory at a time")
            menu(item)
            row = wait(
                "one coherent laboratory",
                lambda row: len(row["laboratories"]) == 1 and row["laboratories"][0]["layout_ready"],
            )
            return row["laboratories"][0]["id"]

        def lab(identity):
            return next((entry for entry in read()["laboratories"] if entry["id"] == identity), None)

        def prefix(identity):
            observed = lab(identity)
            require(observed is not None, "The actual laboratory closed unexpectedly")
            return observed["prefix"]

        def world_point(position):
            row = wait("stable actual world projection", lambda row: row["world_projection_ready"])
            cx, cy, _, _ = row["controls"]["WorldCanvas"]
            wx, wy = row["window_position"]
            ox, oy = row["offset"]
            native.motion(
                native.display, -1,
                round(wx + cx + ox + position[0] * row["zoom"]),
                round(wy + cy + oy + position[1] * row["zoom"]), 0,
            )
            native.button(native.display, 1, 1, 0)
            native.button(native.display, 1, 0, 0)
            native.flush(native.display)
            actions.append({"world_point": position, "monotonic_ns": time.monotonic_ns()})

        wait("initial paused world", lambda row: row["paused"] and row["tick"] == 0)
        native = NativeInput()
        window = native.find_window("Biogenesis-R")
        native.focus(native.display, window, 2, 0)
        native.flush(native.display)
        phase = "empty_world"
        enter("AttemptsInput", "0")
        click("CreateWorldButton")
        wait("actual replacement confirmation", lambda row: row["decision_visible"] and row["decision_controls_ready"] and row["decision"] == "create")
        checkpoint("extra_world_replace_confirmation")
        click("ConfirmWorldDecision")
        wait("actual empty world creation", lambda row: row["paused"] and row["tick"] == 0 and row["population"] == 0)
        checkpoint("extra_empty_world")

        phase = "focused_enter"
        identity = open_lab(1)
        name = prefix(identity)
        click(name + "AddGene")
        wait("actual fixed gene and focused Add button", lambda _row: lab(identity) is not None and len(lab(identity)["state"]["genes"]) == 1 and lab(identity)["focus"] == "AddGene")
        key("Return")
        wait("focused Add consumes Enter without committing", lambda _row: lab(identity) is not None and len(lab(identity)["state"]["genes"]) == 2)
        click(name + "DeleteGene")
        wait("actual extra row deletion", lambda _row: len(lab(identity)["state"]["genes"]) == 1)
        enter(name + "GeneLength", "12")
        wait("LineEdit Enter edits without committing", lambda _row: lab(identity) is not None and lab(identity)["state"]["genes"][0]["length"] == 12)
        require(read()["palettes"][name + "GeneColor"]["count"] == 46, "The original 46 palette entries changed")
        require(lab(identity)["argb_accessibility_name"] and lab(identity)["argb_accessibility_description"], "Advanced ARGB has no accessible description")
        checkpoint("extra_focused_enter")
        click(name + "tab:1")
        enter(name + "Genome_symmetry", "1")
        wait("one-arm native color fixture", lambda _row: lab(identity) is not None and lab(identity)["state"]["properties"]["symmetry"] == 1)
        click(name + "tab:0")

        phase = "invalid_argb"
        original = lab(identity)["state"]["genes"][0]["color"]
        invalid = ["GG000000", "0xFF0000", "+FFFFFFF", " FFFFFF ", "1234567", "123456789"]
        for value in invalid:
            enter(name + "GeneArgb", value)
            wait("invalid ARGB retained without mutation", lambda row: lab(identity) is not None and lab(identity)["error"] and "GeneArgb" in lab(identity)["invalid"] and row["texts"][name + "GeneArgb"] == value)
            require(lab(identity)["state"]["genes"][0]["color"] == original, "Invalid hex mutated the draft")
            click(name + "CommitGenome")
            wait("invalid ARGB blocks actual Commit", lambda _row: lab(identity) is not None and "Correct these entries" in lab(identity)["error"])
        checkpoint("extra_invalid_argb")

        phase = "half_alpha_commit"
        enter(name + "GeneArgb", "80ff0000")
        wait("actual half-red edit and canonical display", lambda row: lab(identity) is not None and lab(identity)["state"]["genes"][0]["color"] == 0x80FF0000 and row["texts"][name + "GeneArgb"] == "80FF0000" and not lab(identity)["invalid"])
        require(read()["palettes"][name + "GeneColor"]["count"] == 47, "Custom ARGB was not preserved alongside the palette")
        wait("unconsumed Enter precondition after editor rebuild", lambda _row: lab(identity)["focus"] == "")
        checkpoint("extra_half_alpha_draft")
        key("Return")
        wait("default Enter commits actual clipboard", lambda row: not row["laboratories"] and row["clipboard_ready"])

        phase = "custom_preservation"
        identity = open_lab(0)
        name = prefix(identity)
        require(lab(identity)["state"]["genes"][0]["color"] == 0x80FF0000, "Clipboard roundtrip lost custom ARGB")
        require(read()["texts"][name + "GeneArgb"] == "80FF0000", "Untouched custom ARGB display changed")
        enter(name + "GeneAngle", "0")
        require(lab(identity) is not None, "Unchanged LineEdit Enter committed the laboratory")
        click(name + "GeneLengthIncrement")
        wait("unrelated edit preserves custom color", lambda _row: lab(identity)["state"]["genes"][0]["length"] == 12.1 and lab(identity)["state"]["genes"][0]["color"] == 0x80FF0000)
        enter(name + "GeneLength", "12")
        wait("unhandled Enter precondition after unrelated edit", lambda _row: lab(identity)["focus"] == "")
        key("Return")
        wait("default Enter commits preserved custom value", lambda row: not row["laboratories"])

        phase = "actual_paste_and_pixels"
        menu(4)
        wait("actual clipboard placement mode", lambda row: row["placement_mode"] == "paste")
        dimensions = read()["world_size"]
        position = [value * 0.5 for value in dimensions]
        world_point(position)
        placed = wait("actual half-red organism pasted", lambda row: row["population"] == 1 and row["packed_count"] > 0)
        require(placed["packed_alphas"] == [128], "Actual one-arm paste lost half alpha")
        require(all(value == 0xFF0000 for value in placed["packed_colors"]), "Actual paste lost red RGB")
        wait("successful paste automatically clears placement mode", lambda row: not row["placement_mode"])
        world_point([dimensions[0] * 0.1, dimensions[1] * 0.1])
        wait("actual empty pick leaves no selection", lambda row: not row["selected"] and row["focus"] == "WorldCanvas")
        pixels = checkpoint("extra_alpha_render")
        require(pixels["alpha_pixels"]["count"] > 0, "No actual nonopaque red pixel was observed")
        require(pixels["paused"] and pixels["tick"] == 0, "Laboratory controls advanced biology")

        phase = "escape_and_exit"
        identity = open_lab(0)
        name = prefix(identity)
        click(name + "GeneArgb")
        key("Escape")
        wait("existing focused Escape cancels laboratory", lambda row: not row["laboratories"])
        checkpoint("extra_escape_restored")
        click("QuitButton")
        wait("actual exit confirmation", lambda row: row["decision_visible"] and row["decision_controls_ready"] and row["decision"] == "quit")
        click("ConfirmWorldDecision")
        require(process.wait(timeout=5) == 0, "Actual native exit failed")
        require(not godot_errors(log.read_text(errors="replace")), "Late laboratory frontend error")
        proof = {
            "result": "PASS", "kind": "laboratory_extra_controls",
            "actions": len(actions), "checkpoints": len(checks),
            "invalid_inputs": invalid, "alpha_pixels": pixels["alpha_pixels"],
            "library_sha256": hashlib.sha256((ROOT / "bin/libbiogenesis_godot.so").read_bytes()).hexdigest(),
            "model_state_injection": False,
            "limits": ["No raw-null UI producer or arbitrary-color XML acceptance", "No timing or full TASK9 acceptance"],
        }
        save(run / "summary.json", proof)
        print(json.dumps(proof), flush=True)
    except BaseException as error:
        save(run / "failure.json", {"phase": phase, "error": str(error), "actions": len(actions), "checks": len(checks)})
        raise
    finally:
        try:
            save(run / "actions.json", actions)
            save(run / "checkpoints.json", checks)
        finally:
            try:
                if native is not None:
                    native.close(native.display)
                if process is not None and process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=2)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=2)
            finally:
                for number, handler in previous.items():
                    signal.signal(number, handler)
