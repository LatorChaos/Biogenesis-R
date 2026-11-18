"""Exercise frozen PNG exports through real XTest input on an owned display"""

import hashlib
import json
import math
import os
from pathlib import Path
import signal
import subprocess
import time

from build import ROOT, godot_errors
from check_native_controls import NativeInput


def image_inside(engine, run):
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
    proof = {}
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
                    "--script", "res://ui/image_export_controls_check.gd",
                    "--resolution", "1200x900",
                ],
                stdout=output,
                stderr=subprocess.STDOUT,
            )

        offset = 0
        partial = b""
        latest_row = None
        frozen_before = None
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
                require(time.monotonic() - started < 180, "Image journey exceeded 180 seconds")
                row = read()
                if row and predicate(row):
                    return row
                require(process.poll() is None, "Frontend exited before " + description)
                time.sleep(0.015)
            row = read()
            detail = None if row is None else {
                key: row.get(key) for key in
                ("tick", "paused", "selected", "file_operation", "file_dialog",
                 "overwrite", "image_pending", "application_error")
            }
            raise ValueError("Timed out " + description + "; observed=" + repr(detail))

        def key(name):
            native.key(name)
            actions.append({"key": name, "monotonic_ns": time.monotonic_ns()})

        def click(name):
            row = wait(
                "stable actual control " + name,
                lambda observed: observed["control_ready"].get(name, False),
            )
            native.click(row, name)
            actions.append({"control": name, "monotonic_ns": time.monotonic_ns()})

        def enter(name, value, submit=True, field="FileName"):
            click(name)
            native.set_key("Control_L", True)
            try:
                key("a")
            finally:
                native.set_key("Control_L", False)
            symbols = {"/": "slash", ".": "period", "-": "minus", " ": "space"}
            shifted = {"#": "3", ":": "semicolon", "_": "minus"}
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
            wait(
                "actual filename text equals requested input",
                lambda row: row["file_dialog"].get("visible", False)
                and row["file_dialog"].get(field) == name
                and row["file_dialog"].get("filename_text" if field == "FileName" else "directory_text") == str(value),
            )
            if submit:
                key("Return")
            actions.append({"entry": name, "value": str(value), "monotonic_ns": time.monotonic_ns()})

        def choose_filter(kind):
            row = read()
            labels = row["file_dialog"]["filter"]["labels"]
            matches = [index for index, label in enumerate(labels) if label.lower().startswith(kind)]
            require(matches, "Actual chooser filter is absent: " + kind)
            index = matches[0]
            require(index < 4, "Actual filter exceeds the bounded image chooser choices")
            require(0 < len(labels) <= 4, "Actual filter popup exceeds its bounded navigation count")
            click(row["file_dialog"]["filter"]["control"])
            row = wait("actual filter popup opens", lambda value: value["file_dialog"]["filter"]["open"])
            for _ in range(len(labels)):
                focused = row["file_dialog"]["filter"]["focused"]
                if focused == index:
                    break
                key("Down")
                row = wait("actual filter keyboard focus advances", lambda value: value["file_dialog"]["filter"]["focused"] != focused)
                require(0 <= row["file_dialog"]["filter"]["focused"] < len(labels), "Actual filter keyboard focus is outside its popup")
            require(row["file_dialog"]["filter"]["focused"] == index, "Actual filter keyboard navigation did not reach its requested choice")
            key("Return")
            return wait("actual filter selection", lambda value: not value["file_dialog"]["filter"]["open"] and value["file_dialog"]["filter"]["selected"] == index)

        def approve(method):
            row = read()
            text = row["file_dialog"]["filename_text"]
            path = Path(text)
            if not path.is_absolute():
                path = Path(row["file_dialog"]["directory"]) / path
            require(text and path.name, "Actual approved filename must be nonempty")
            proof.setdefault("approved_names", []).append({
                "visible_filename": text, "path": str(path), "acceptance": method,
                "filter": row["file_dialog"]["filter"]["labels"][row["file_dialog"]["filter"]["selected"]],
            })
            if method == "Return":
                key("Return")
            else:
                click(row["file_dialog"]["FileOkay"])
            return path

        def select_existing(name):
            row = read()
            items = row["file_dialog"]["file_list"]["items"]
            require(items.count(name) == 1, "Owned existing filename is not uniquely listed")
            target = items.index(name)
            prefix_matches = [index for index, item in enumerate(items) if item.startswith(name)]
            require({items[index] for index in prefix_matches} == {name, name + ".png"} and len(prefix_matches) == 2, "Owned exact-name and suffix sibling search inventory differs")
            require(name == "native_default_bare", "Existing-file search must use its bounded owned basename")
            click(row["file_dialog"]["file_list"]["control"])
            for character in name:
                if character == "_":
                    native.set_key("Shift_L", True)
                    try:
                        key("minus")
                    finally:
                        native.set_key("Shift_L", False)
                else:
                    key(character)
            row = wait("actual filename-search selection", lambda value: len(value["file_dialog"]["file_list"]["selected"]) == 1 and value["file_dialog"]["file_list"]["selected"][0] in prefix_matches)
            selected_index = row["file_dialog"]["file_list"]["selected"][0]
            if selected_index != target:
                key("Up" if selected_index > target else "Down")
                wait("actual exact-file search disambiguation", lambda value: value["file_dialog"]["file_list"]["selected"] == [target])
            return wait("existing file replaces prior typed name", lambda value: value["file_dialog"]["file_list"]["selected"] == [target] and value["file_dialog"]["filename_text"] == name)

        def checkpoint(label, path=None, identity=None):
            request = {"label": label}
            if path is not None:
                request.update(image_path=str(path), export_id=identity)
            pending = run / "checkpoint-request.pending"
            save(pending, request)
            pending.replace(run / "checkpoint-request.json")
            wait(
                "image checkpoint " + label,
                lambda _row: any(row["label"] == label for row in checkpoint_rows),
            )
            row = next(row for row in checkpoint_rows if row["label"] == label)
            if path is not None:
                require(row["image_check"]["png_sha256"] == digest(path), "PNG changed after exact native readback")
            checks.append(row)
            return row

        def digest(path):
            return hashlib.sha256(path.read_bytes()).hexdigest()

        def world_point(position, button=1):
            row = read()
            x, y, _, _ = row["controls"]["WorldCanvas"]
            wx, wy = row["window_position"]
            ox, oy = row["offset"]
            native.motion(
                native.display, -1,
                round(wx + x + ox + position[0] * row["zoom"]),
                round(wy + y + oy + position[1] * row["zoom"]), 0,
            )
            native.button(native.display, button, 1, 0)
            native.button(native.display, button, 0, 0)
            native.flush(native.display)
            actions.append({"world_point": position, "button": button, "monotonic_ns": time.monotonic_ns()})

        def menu():
            click("GeneticsMenu")
            wait("actual genetics menu", lambda row: row["genetics_menu"])
            for _ in range(16):
                focused = read()["genetics_focused_id"]
                if focused == 8:
                    break
                key("Down")
                wait("actual menu focus change", lambda row: row["genetics_focused_id"] != focused)
            require(read()["genetics_focused_id"] == 8, "Save image menu8 did not receive focus")
            key("Return")
            wait("actual genetics menu closes", lambda row: not row["genetics_menu"])

        def chooser():
            nonlocal frozen_before
            row = wait(
                "paused image chooser with actual frozen token",
                lambda observed: observed["file_dialog"]["visible"]
                and observed["file_operation"] == "image"
                and observed["paused"] and observed["image_pending"],
            )
            identity = row["image_export_id"]
            require(identity in row["image_exports"], "Actual pending token was not observed")
            first_tick = row["tick"]
            stable = wait(
                "chooser holds the actual world and token",
                lambda observed: observed["image_exports"][identity]["observed_frames"] >= 4,
            )
            require(
                stable["tick"] == first_tick and stable["image_export_id"] == identity,
                "Chooser advanced the world or replaced its image token",
            )
            fields = ("tick", "population", "oxygen", "carbon_dioxide", "methane", "selected")
            require(all(stable[field] == row[field] for field in fields), "Paused chooser changed actual World or selection state")
            frozen_before = {field: stable[field] for field in fields}
            return stable, identity

        def finish(paused):
            nonlocal frozen_before
            row = wait(
                "file operation restores its prior activity and releases token",
                lambda row: not row["file_operation"] and not row["image_pending"]
                and row["paused"] is paused,
            )
            if paused and frozen_before is not None:
                require(all(row[field] == value for field, value in frozen_before.items()), "Image completion changed paused World or selection state")
            frozen_before = None
            return row

        ready = wait("initial paused populated world", lambda row: row["paused"] and row["tick"] == 0 and row["population"] > 0)
        native = NativeInput()
        window = native.find_window("Biogenesis-R")
        native.focus(native.display, window, 2, 0)
        native.flush(native.display)
        position = next(
            item["position"] for item in ready["organism_points"]
            if 8 < ready["offset"][0] + item["position"][0] * ready["zoom"] < ready["controls"]["WorldCanvas"][2] - 8
            and 8 < ready["offset"][1] + item["position"][1] * ready["zoom"] < ready["controls"]["WorldCanvas"][3] - 8
        )
        for _ in range(14):
            zoom = read()["zoom"]
            if zoom >= 2:
                break
            world_point(position, 4)
            wait("actual selection zoom", lambda row: row["zoom"] > zoom)
        world_point(position)
        selected = wait("actual living selection", lambda row: row["selected"] and row["selected"]["alive"])
        proof["selected_id"] = selected["selected"]["id"]
        checkpoint("image_selected")

        phase = "paused_context_cancel"
        bounds = selected["selected"]["bounds"]
        world_point([bounds[0] + bounds[2] / 2, bounds[1] + bounds[3] / 2], 3)
        wait("actual organism context menu", lambda row: row["organism_menu"]["visible"])
        for _ in range(20):
            focused = read()["organism_menu"]["focused_action"]
            if focused == "image":
                break
            key("Down")
            wait("actual organism menu focus change", lambda row: row["organism_menu"]["focused_action"] != focused)
        require(read()["organism_menu"]["focused_action"] == "image", "Actual Save image context action is absent")
        key("Return")
        row, _identity = chooser()
        checkpoint("image_context_chooser")
        click(row["file_dialog"]["FileCancel"])
        finish(True)

        phase = "exact_name_matrix"
        cases = [
            ("default_bare", None, "native_default_bare", "Return", True),
            ("default_other", None, "native_default.other", "Save", True),
            ("png_bare", "png", "native_png_bare", "Save", True),
            ("png_other", "png", "native_png.other", "Return", True),
            ("all_bare", "all files", "native_all_bare", "Return", True),
            ("all_other", "all files", "native_all.other", "Save", False),
            ("filter_after_typing", None, "native_changed.other", "Save", True),
        ]
        proof["filename_matrix"] = []
        for label, filter_kind, filename, method, paused in cases:
            if not paused:
                tick = read()["tick"]
                click("PauseButton")
                wait("matrix running world", lambda row: not row["paused"] and row["tick"] > tick)
            menu()
            row, identity = chooser()
            require(row["file_dialog"]["filter"]["labels"][0].lower().startswith("all recognized"), "Default filter label differs")
            if label.startswith("default_"):
                require(row["file_dialog"]["filter"]["selected"] == 0, "Default All Recognized filter was not retained")
            elif label == "filter_after_typing":
                row = choose_filter("all recognized")
            if Path(row["file_dialog"]["directory"]) != run:
                enter(row["file_dialog"]["Directory"], run, True, "Directory")
                row = wait("actual owned image directory", lambda value: Path(value["file_dialog"]["directory"]) == run)
            if filter_kind is not None:
                row = choose_filter(filter_kind)
            if label == "png_other":
                enter(row["file_dialog"]["FileName"], run / "native_prior.other", False)
                choose_filter("all files")
                row = choose_filter("png")
            enter(row["file_dialog"]["FileName"], run / filename, False)
            if label == "filter_after_typing":
                choose_filter("png")
            approved = approve(method)
            resumed = finish(paused)
            require(approved.is_file(), "PNG was not saved to the exact visible approved filename")
            sibling = Path(str(approved) + ".png")
            require(not sibling.exists(), "An automatic PNG suffix changed the approved destination")
            require(resumed["selected"]["id"] == selected["selected"]["id"], "Image save changed selected identity")
            checkpoint("image_exact_" + label, approved, identity)
            proof["filename_matrix"].append({"case": label, "path": str(approved), "acceptance": method, "paused": paused})
            if not paused:
                wait("matrix save restores actual ticks", lambda row: row["tick"] > resumed["tick"])
                click("PauseButton")
                wait("matrix returns to pause", lambda row: row["paused"])

        phase = "exact_overwrite"
        path = run / "native_default_bare"
        sibling = run / "native_default_bare.png"
        require(path.is_file() and not sibling.exists(), "Preserve actual exact-name matrix output")
        with (run / "native-image-before-overwrite.png").open("xb") as original:
            original.write(path.read_bytes())
        path.write_bytes(b"Owned exact destination bytes for cancel and confirm\n")
        with sibling.open("xb") as output:
            output.write(b"Owned suffix sibling must remain unchanged\n")
        prior_sha256, sibling_sha256 = digest(path), digest(sibling)
        proof["overwrite_prior_fixture"] = {"path": str(path), "sha256": prior_sha256, "sibling": str(sibling), "sibling_sha256": sibling_sha256}
        for confirm in (False, True):
            menu()
            row, identity = chooser()
            row = choose_filter("png" if confirm else "all files")
            enter(row["file_dialog"]["FileName"], path if confirm else run / "native_stale.other", False)
            if not confirm:
                select_existing(path.name)
            approved = approve("Save" if confirm else "Return")
            require(approved == path, "Existing selection did not replace the earlier typed destination")
            wait("exact-target overwrite confirmation", lambda observed: observed["overwrite"] and observed["save_path"] == str(path))
            checkpoint("image_overwrite_confirm" if confirm else "image_overwrite_cancel")
            click("OverwriteOkay" if confirm else "OverwriteCancel")
            finish(True)
            require(digest(sibling) == sibling_sha256, "Overwrite changed the differently suffixed sibling")
            if confirm:
                checkpoint("image_confirmed_saved", path, identity)
            else:
                require(digest(path) == prior_sha256, "Overwrite cancel changed exact destination bytes")

        phase = "running_cancel"
        tick = read()["tick"]
        click("PauseButton")
        wait("actual running world", lambda row: not row["paused"] and row["tick"] > tick)
        menu()
        chooser()
        key("Escape")
        resumed = finish(False)
        wait("Escape cancel resumes actual ticks", lambda row: row["tick"] > resumed["tick"])
        require(process.poll() is None, "Chooser Escape quit the application")
        click("PauseButton")
        wait("actual pause before destination failure", lambda row: row["paused"])

        phase = "destination_error"
        missing = run / "owned-missing-parent" / "native_error"
        require(not missing.parent.exists(), "Preserve the missing-parent error fixture")
        menu()
        row, _identity = chooser()
        row = choose_filter("all files")
        enter(row["file_dialog"]["FileName"], missing, False)
        require(approve("Return") == missing, "Destination error used a different approved name")
        failed = finish(True)
        require(failed["application_error"] and failed["selected"]["id"] == selected["selected"]["id"], "Handled destination failure lost its error or selection")
        require(not missing.exists() and not missing.parent.exists(), "Destination failure created an owned path")
        checkpoint("image_destination_error")

        phase = "error_focus_and_exit"
        wait("actual pause before deselection", lambda row: row["paused"])
        row = read()
        width, height = row["controls"]["WorldCanvas"][2:]
        empty = None
        for fx, fy in ((0.1, 0.1), (0.9, 0.1), (0.1, 0.9), (0.9, 0.9), (0.5, 0.5)):
            point = [(width * fx - row["offset"][0]) / row["zoom"], (height * fy - row["offset"][1]) / row["zoom"]]
            if all(0 < point[axis] < row["world_size"][axis] for axis in (0, 1)) and all(
                math.dist(point, item["position"]) > math.hypot(*item["extent"]) / 2 + 25
                for item in row["organism_points"]
            ):
                empty = point
                break
        require(empty is not None, "No bounded actual empty world point is available")
        world_point(empty)
        wait("actual world focus and deselection", lambda row: not row["selected"] and row["focus"] == "WorldCanvas")
        menu()
        wait("paused preparation error releases token", lambda row: row["application_error"] and not row["file_dialog"]["visible"])
        finish(True)
        click("PauseButton")
        wait("running without selection", lambda row: not row["paused"])
        menu()
        resumed = finish(False)
        require(resumed["application_error"] and not resumed["file_dialog"]["visible"], "Preparation failure lost its visible error")
        wait("preparation error restores actual ticks", lambda row: row["tick"] > resumed["tick"])
        click("PauseButton")
        finish(True)
        checkpoint("image_error_recovered")
        click("QuitButton")
        wait("actual exit confirmation", lambda row: row["decision_visible"] and row["decision_controls_ready"] and row["decision"] == "quit")
        click("ConfirmWorldDecision")
        require(process.wait(timeout=5) == 0, "Actual native exit failed")
        require(not godot_errors(log.read_text(errors="replace")), "Late image frontend error")
        require(len(cases) == 7 and len(checks) == 14, "Exact image matrix/checkpoint count differs")
        require(len([row for row in checks if row["image_check"]]) == 8, "Exact saved-image readback count differs")
        require(len(read()["image_exports"]) == 12 and len(actions) <= 418, "Image token/action allocation exceeded its bound")
        proof.update(
            result="PASS", kind="image_export_controls", actions=len(actions),
            checkpoints=len(checks), model_state_injection=False,
            saved_images=[row["image_check"] for row in checks if row["image_check"]],
            exact_name_matrix_cases=len(cases), prepared_tokens=len(read()["image_exports"]),
            limits=["No timing or full TASK9 acceptance", "No native alpha-import claim"],
        )
        save(run / "summary.json", proof)
        print(json.dumps({"result": "PASS", "kind": "image_export_controls", "actions": len(actions), "checkpoints": len(checks)}), flush=True)
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
