"""Exercise laboratory controls with real XTest input on an owned display"""

import hashlib
import json
import math
import os
from pathlib import Path
import re
import signal
import subprocess
import time
import xml.etree.ElementTree as ET

from build import ROOT, godot_errors
from check_native_controls import NativeInput


STORAGE_REACTIONS = "red green blue plague scourge white gray silver default consumer plant magenta pink coral orange bark violet virus maroon olive mint cream spike fallow lightblue ochre sky lilac fire lightbrown greenbrown brown ice broken sick friend".split()
BOOLEAN_PROPERTIES = "plague disperse_children generation_battle sibling_battle altruist familial social peaceful passive clockwise mimic_all modifies_pink modifies_lilac modifies_sky selfish".split()
NUMERIC_PROPERTIES = {"symmetry": 3, "mirror": 1, "mutation_rate": 37, "activity": 1,
                      "cream": 3, "fallow": 3, "spore": 11, "adapt_spore": 51}
ROOT_ATTRIBUTES = {"mutation_rate": "mutationrate", "disperse_children": "disperse",
                   "generation_battle": "generation", "sibling_battle": "sibling",
                   "mimic_all": "mimicall", "modifies_pink": "modifiespink",
                   "modifies_lilac": "modifieslilac", "modifies_sky": "modifiessky",
                   "cream": "modifiescream", "fallow": "modifiesfallow", "spore": "modifiesspore",
                   "adapt_spore": "adaptspore"}


def laboratory_inside(engine, run):
    def require(value, message):
        if not value:
            raise ValueError(message)

    def save(path, value):
        path.write_text(json.dumps(value, indent=2) + "\n")

    def stop(number, _frame):
        raise SystemExit(128 + number)

    previous_handlers = {number: signal.signal(number, stop) for number in (signal.SIGTERM, signal.SIGINT)}
    native, process = None, None
    actions, checks, proof = [], [], {}
    phase = "startup"
    phase_started = time.monotonic()
    log = run / "engine.log"
    try:
        require(os.environ.get("BGR_TEST_ROOT") == str(run) and os.environ.get("DISPLAY", "").startswith(":"),
                "Owned display and run directory are required")
        require(run.is_dir() and not log.exists(), "Preserve the existing native attempt")
        color_path = ROOT / "tests/fixtures/java/core.jsonl"
        colors = None
        for line in color_path.read_text().splitlines():
            observation = json.loads(line)
            if observation["scenario"] == "color_identities":
                colors = [entry for entry in observation["observations"]["colors"] if entry["selectable"]]
        require(colors is not None and len(colors) == 46, "Frozen original palette observation is required")
        proof["palette_input_sha256"] = hashlib.sha256(color_path.read_bytes()).hexdigest()
        with log.open("xb") as output:
            process = subprocess.Popen([str(engine), "--audio-driver", "Dummy", "--path", str(ROOT),
                                        "--script", "res://ui/laboratory_controls_check.gd", "--resolution", "1200x900"],
                                       stdout=output, stderr=subprocess.STDOUT)

        log_offset, partial = 0, b""
        observed_lines = {"BGR_CRISP_OBS": [], "BGR_CRISP_CHECKPOINT": []}

        def lines(prefix):
            nonlocal log_offset, partial
            with log.open("rb") as source:
                source.seek(log_offset)
                pending = partial + source.read()
                log_offset = source.tell()
            complete_bytes, _, partial = pending.rpartition(b"\n")
            complete = complete_bytes.decode("utf-8", errors="replace")
            require("BGR_GUI_FAIL" not in complete and not godot_errors(complete), "Frontend error; inspect engine.log")
            for line in complete.splitlines():
                for marker in observed_lines:
                    if line.startswith(marker + " "):
                        row = json.loads(line[len(marker) + 1:])
                        if marker == "BGR_CRISP_OBS":
                            observed_lines[marker] = [row]
                        else:
                            observed_lines[marker].append(row)
            return observed_lines[prefix]

        def latest():
            records = lines("BGR_CRISP_OBS")
            return records[-1] if records else None

        def concise(row):
            if not row:
                return None
            return {"tick": row["tick"], "paused": row["paused"], "population": row["population"],
                    "labs": [{"id": lab["id"], "tab": lab["tab"], "genes": len(lab["state"].get("genes", [])),
                              "focus": lab["focus"], "has_focus": lab["has_focus"], "embedded": lab["embedded"],
                              "exclusive": lab["exclusive"], "popup": lab["popup"], "transient": lab["transient"],
                              "layout_ready": lab["layout_ready"], "stable_frames": lab["layout_stable_frames"],
                              "error": lab["error"]} for lab in row["laboratories"]],
                    "root_has_focus": row["root_has_focus"], "canvas_focus": row["focus"],
                    "root_mouse": row["root_mouse"], "last_world_pick": row["last_world_pick"],
                    "file": row["file_operation"], "error": row["application_error"]}

        def wait(description, predicate, timeout=12):
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                require(time.monotonic() - phase_started <= 130, "Native phase exceeded 130 seconds: " + phase)
                observed = latest()
                if observed and predicate(observed):
                    return observed
                require(process.poll() is None, "Frontend exited before " + description)
                time.sleep(0.015)
            raise ValueError("Timed out " + description + "; observed=" + repr(concise(latest())))

        def begin(name):
            nonlocal phase, phase_started
            phase, phase_started = name, time.monotonic()
            actions.append({"phase": name, "monotonic_ns": time.monotonic_ns()})

        def key(name):
            native.key(name)
            actions.append({"key": name, "monotonic_ns": time.monotonic_ns()})

        def chord(modifier, name):
            native.set_key(modifier, True)
            key(name)
            native.set_key(modifier, False)
            native.flush(native.display)

        def text(value):
            symbols = {"/": "slash", ".": "period", "-": "minus", " ": "space"}
            shifted = {"#": "3", ":": "semicolon", "_": "minus"}
            for character in str(value):
                if character.isupper() or character in shifted:
                    native.set_key("Shift_L", True)
                    native.key(character.lower() if character.isupper() else shifted[character])
                    native.set_key("Shift_L", False)
                else:
                    native.key(symbols.get(character, character))
            native.flush(native.display)

        def point_screen(x, y, button=1):
            native.motion(native.display, -1, round(x), round(y), 0)
            native.button(native.display, button, 1, 0)
            native.button(native.display, button, 0, 0)
            native.flush(native.display)

        def visible(name):
            for _ in range(70):
                if name.startswith("lab:"):
                    identity = name.split(":")[1]
                    wait("coherent laboratory layout before " + name,
                         lambda row: any(lab["id"] == identity and lab["layout_ready"] for lab in row["laboratories"]))
                observed = latest()
                require(name in observed["controls"], "Actual control is absent: " + name)
                detail = observed["control_details"].get(name)
                if detail is None or detail["inside"]:
                    return wait("stable positive control rectangle " + name,
                                lambda row: row["control_ready"].get(name, False))
                require(detail["scroll"], "Control is outside its window: " + name)
                rect = observed["controls"][name]
                clip = detail["clip"]
                direction = 5 if rect[1] + rect[3] > clip[1] + clip[3] else 4
                native.point_at(observed, detail["scroll"])
                native.button(native.display, direction, 1, 0)
                native.button(native.display, direction, 0, 0)
                native.flush(native.display)
                time.sleep(0.025)
            raise ValueError("Native wheel did not reveal " + name)

        def click(name):
            observed = visible(name)
            native.click(observed, name)
            actions.append({"control": name, "monotonic_ns": time.monotonic_ns()})

        def enter(name, value):
            click(name)
            chord("Control_L", "a")
            text(value)
            wait("actual entered text " + name, lambda row: row["texts"].get(name) == str(value))
            key("Return")
            actions.append({"entry": name, "value": str(value), "monotonic_ns": time.monotonic_ns()})

        def laboratory(identity):
            return next((lab for lab in latest()["laboratories"] if lab["id"] == identity), None)

        def prefix(identity):
            lab = laboratory(identity)
            require(lab is not None, "Expected laboratory closed")
            return lab["prefix"]

        def lab_state(identity):
            return laboratory(identity)["state"]

        def checkpoint(label):
            pending = run / "checkpoint-request.pending"
            save(pending, {"label": label})
            pending.replace(run / "checkpoint-request.json")
            until = time.monotonic() + 12
            while time.monotonic() < until:
                for row in lines("BGR_CRISP_CHECKPOINT"):
                    if row["label"] == label:
                        checks.append(row)
                        return row
                require(process.poll() is None, "Frontend exited before checkpoint " + label)
                time.sleep(0.02)
            raise ValueError("Missing laboratory checkpoint " + label)

        def main_focus():
            native.focus(native.display, main_window, 2, 0)
            native.flush(native.display)

        def menu(identity):
            main_focus()
            click("GeneticsMenu")
            wait("genetics popup", lambda row: row["genetics_menu"])
            for _ in range(16):
                focused = latest()["genetics_focused_id"]
                if focused == identity:
                    break
                key("Down")
                wait("actual genetics focus changes", lambda row: row["genetics_focused_id"] != focused)
            require(latest()["genetics_focused_id"] == identity, "Requested genetics item did not receive actual focus")
            key("Return")
            wait("genetics popup closed", lambda row: not row["genetics_menu"])

        def open_lab(source):
            before = {lab["id"] for lab in latest()["laboratories"]}
            menu(source)
            observed = wait("new actual laboratory layout", lambda row: len(row["laboratories"]) == len(before) + 1 and
                            all(lab["layout_ready"] for lab in row["laboratories"] if lab["id"] not in before))
            return next(lab["id"] for lab in observed["laboratories"] if lab["id"] not in before)

        def open_second_lab():
            wait("first laboratory layout before parent activation",
                 lambda row: all(lab["layout_ready"] for lab in row["laboratories"]))
            observed = latest()
            require(not observed["clipboard_ready"], "Second-window isolation requires the original empty clipboard")
            before = {lab["id"] for lab in observed["laboratories"]}
            require(len(before) == 1, "One actual laboratory must precede the second window")
            cx, cy, width, height = observed["controls"]["WorldCanvas"]
            candidate = None
            for local_x, local_y in ((8, 8), (width - 8, 8), (8, height / 2), (width - 8, height / 2),
                                      (8, height - 8), (width - 8, height - 8)):
                x, y = cx + local_x, cy + local_y
                if any(lab["position"][0] - 10 <= x <= lab["position"][0] + lab["size"][0] + 10 and
                       lab["position"][1] - 32 <= y <= lab["position"][1] + lab["size"][1] + 10
                       for lab in observed["laboratories"]):
                    continue
                point = [(local_x - observed["offset"][0]) / observed["zoom"],
                         (local_y - observed["offset"][1]) / observed["zoom"]]
                if all(math.dist(point, item["position"]) > math.hypot(*item["extent"]) / 2 + 8 / observed["zoom"]
                       for item in observed["organism_points"]):
                    candidate = point
                    break
            require(candidate is not None, "No empty canvas point remains outside the laboratory rectangle")
            main_focus()
            actions.append({"precondition": "native_main_focus_before_canvas_click", "monotonic_ns": time.monotonic_ns()})
            world_point(candidate, 1)
            wait("actual parent canvas focus and left-click callback",
                 lambda row: row["root_has_focus"] and row["focus"] == "WorldCanvas" and
                 math.dist(row["last_world_pick"], candidate) * row["zoom"] <= 0.5)
            world_point(candidate, 3)
            wait("actual empty-world popup", lambda row: row["world_context"]["visible"])
            for _ in range(16):
                focused = latest()["world_context"]["focused_id"]
                if focused == 0:
                    break
                key("Down")
                wait("actual empty-world focus changes", lambda row: row["world_context"]["focused_id"] != focused)
            require(latest()["world_context"]["focused_id"] == 0, "Actual laboratory popup item did not receive focus")
            key("Return")
            after = wait("second actual laboratory layout", lambda row: not row["world_context"]["visible"] and len(row["laboratories"]) == 2 and
                         all(lab["layout_ready"] for lab in row["laboratories"] if lab["id"] not in before))
            return next(lab["id"] for lab in after["laboratories"] if lab["id"] not in before)

        def close_lab(identity, button="CancelLaboratory"):
            click(prefix(identity) + button)
            wait("laboratory closes", lambda row: all(lab["id"] != identity for lab in row["laboratories"]))

        def distinct_clipboard(original_genes):
            temporary = open_lab(1)
            click(prefix(temporary) + "AddGene")
            wait("distinct fixed clipboard gene", lambda _row: len(lab_state(temporary)["genes"]) == 1)
            close_lab(temporary, "CommitGenome")
            readback = open_lab(0)
            require(lab_state(readback)["genes"] != original_genes, "Clipboard replacement did not produce distinct values")
            close_lab(readback)

        def switch_tab(identity, index):
            click(prefix(identity) + "tab:" + str(index))
            wait("laboratory tab", lambda _row: laboratory(identity)["tab"] == index)

        def edit_gene(identity, field, value, test):
            enter(prefix(identity) + field, value)
            wait("actual gene edit " + field, lambda _row: test(lab_state(identity)["genes"][laboratory(identity)["selected_gene"]]))

        def palette_choice(identity, index, expected_argb, label):
            name = prefix(identity) + "GeneColor"
            click(name)
            wait("actual palette opens", lambda row: row["palettes"][name]["open"])
            for _ in range(48):
                focused = latest()["palettes"][name]["focused_index"]
                if focused == index:
                    break
                key("Down")
                wait("actual palette focus changes", lambda row: row["palettes"][name]["open"] and
                     row["palettes"][name]["focused_index"] >= 0 and row["palettes"][name]["focused_index"] != focused)
            require(latest()["palettes"][name]["focused_index"] == index, "Source palette target did not receive actual focus")
            key("Return")
            wait("source palette " + label, lambda row: not row["palettes"][name]["open"] and
                 row["palettes"][name]["selected"] == index and lab_state(identity)["genes"][0]["color"] == expected_argb)

        def world_point(position, button=1):
            row = wait("three-frame world projection before native input", lambda row: row["world_projection_ready"])
            cx, cy, _, _ = row["controls"]["WorldCanvas"]
            wx, wy = row["window_position"]
            ox, oy = row["offset"]
            x, y = wx + cx + ox + position[0] * row["zoom"], wy + cy + oy + position[1] * row["zoom"]
            point_screen(x, y, button)
            actions.append({"world_point": position, "button": button, "monotonic_ns": time.monotonic_ns()})

        def empty_world_point():
            row = latest()
            _, _, width, height = row["controls"]["WorldCanvas"]
            world_width, world_height = row["world_size"]
            for fx, fy in ((0.1, 0.1), (0.9, 0.1), (0.1, 0.9), (0.9, 0.9), (0.5, 0.5)):
                point = [world_width * fx, world_height * fy]
                screen = [row["offset"][i] + point[i] * row["zoom"] for i in range(2)]
                if not (5 < screen[0] < width - 5 and 5 < screen[1] < height - 5):
                    continue
                if all(math.dist(point, item["position"]) > math.hypot(*item["extent"]) / 2 + 25
                       for item in row["organism_points"]):
                    return point
            raise ValueError("No bounded empty canvas point was found")

        def select_world(position):
            for _ in range(14):
                if latest()["zoom"] >= 2:
                    break
                before = latest()["zoom"]
                world_point(position, 4)
                wait("actual selection zoom", lambda row: row["zoom"] > before)
            world_point(position)
            return wait("actual selected organism", lambda row: bool(row["selected"]))

        def file_path(path, dialog=None):
            state = latest()["file_dialog"] if dialog is None else laboratory(dialog)["import_dialog"]
            require(state["visible"], "Actual genotype file dialog is absent")
            enter(state["FileName"], str(path))

        def xml_readback(path, expected):
            require(path.is_file(), "Actual export file is absent")
            document = ET.fromstring(path.read_bytes())
            require(document.tag == "genetic_code", "Wrong exported genotype root")
            properties = expected["properties"]
            require(len(document.attrib) == 23, "Export lost a public genotype field")
            for name, value in properties.items():
                attribute = ROOT_ATTRIBUTES.get(name, name)
                actual = document.attrib[attribute]
                target = ("yes" if value else "no") if isinstance(value, bool) or name == "mirror" else str(value)
                require(actual == target, "Export metadata differs: " + name)
            genes = document.findall("gene")
            require(len(genes) == len(expected["genes"]), "Export gene count differs")
            for actual, gene in zip(genes, expected["genes"]):
                require(len(actual.attrib) == 40, "Export lost a Gene field")
                require(float(actual.attrib["length"]) == gene["length"] and float(actual.attrib["theta"]) == gene["theta"], "Export geometry differs")
                require(int(actual.attrib["branch"]) == gene["branch"], "Export branch differs")
                require(actual.attrib["color"] == "green", "Export lost the final source GREEN color")
                for index, channel in enumerate(STORAGE_REACTIONS):
                    require(int(actual.attrib[channel + "reaction"]) == gene["reactions"][index], "Export reaction differs: " + channel)
            return hashlib.sha256(path.read_bytes()).hexdigest()

        ready = wait("initial laboratory journey", lambda row: row["paused"] and row["tick"] == 0 and not row["laboratories"])
        native = NativeInput()
        main_window = native.find_window("Biogenesis-R")
        main_focus()
        checkpoint("lab_initial")

        begin("nonmodal_and_isolation")
        click("PauseButton")
        running = wait("running before laboratory", lambda row: not row["paused"] and row["tick"] > 0)
        active = open_lab(1)
        require(not lab_state(active)["genes"], "Empty laboratory generated genes")
        wait("world continues behind nonmodal laboratory", lambda row: row["tick"] >= running["tick"] + 3 and not row["paused"])
        checkpoint("lab_nonmodal")
        close_lab(active)
        click("PauseButton")
        paused = wait("pause for deterministic editing", lambda row: row["paused"])
        stable_tick = paused["tick"]
        first = open_lab(1)
        click(prefix(first) + "AddGene")
        wait("fixed Add", lambda _row: len(lab_state(first)["genes"]) == 1)
        fixed = lab_state(first)["genes"][0]
        require(fixed["length"] == 2 and fixed["theta"] == 0 and fixed["branch"] == -1 and fixed["color"] == 0xff00ff00 and fixed["reactions"] == [0] * 36, "Add differs from the source fixed Gene")
        second = open_second_lab()
        require(not lab_state(second)["genes"] and len(lab_state(first)["genes"]) == 1, "Laboratory windows share local edits")
        close_lab(second)
        click(prefix(first) + "InsertGene")
        wait("fixed Insert", lambda _row: len(lab_state(first)["genes"]) == 2)
        click(prefix(first) + "DeleteGene")
        wait("Delete one row", lambda _row: len(lab_state(first)["genes"]) == 1)
        click(prefix(first) + "ClearGenes")
        wait("Clear local genes", lambda _row: not lab_state(first)["genes"])
        click(prefix(first) + "AddGene")
        wait("Add after Clear", lambda _row: len(lab_state(first)["genes"]) == 1)
        switch_tab(first, 1)
        original_symmetry = lab_state(first)["properties"]["symmetry"]
        enter(prefix(first) + "Genome_symmetry", 9)
        wait("invalid general property is retained visibly", lambda row: laboratory(first)["error"] and
             row["texts"][prefix(first) + "Genome_symmetry"] == "9" and
             lab_state(first)["properties"]["symmetry"] == original_symmetry)
        click(prefix(first) + "ClearGenes")
        wait("Clear removes only genes", lambda _row: not lab_state(first)["genes"])
        switch_tab(first, 0)
        click(prefix(first) + "AddGene")
        wait("Add after invalid-property Clear", lambda _row: len(lab_state(first)["genes"]) == 1)
        switch_tab(first, 1)
        click(prefix(first) + "CommitGenome")
        wait("invalid general property blocks commit after Clear and Add",
             lambda row: bool(laboratory(first)) and bool(laboratory(first)["error"]) and
             row["texts"][prefix(first) + "Genome_symmetry"] == "9" and not row["clipboard_ready"])
        require(lab_state(first)["properties"]["symmetry"] == original_symmetry,
                "Invalid property changed the model through Clear or Commit")
        checkpoint("lab_invalid_general_survives_clear")
        enter(prefix(first) + "Genome_symmetry", original_symmetry)
        wait("corrected general property", lambda row: not laboratory(first)["error"] and
             row["texts"][prefix(first) + "Genome_symmetry"] == str(original_symmetry))

        begin("all_fields_and_palette")
        switch_tab(first, 1)
        for field, value in NUMERIC_PROPERTIES.items():
            enter(prefix(first) + "Genome_" + field, value)
            wait("root property " + field, lambda _row, f=field, v=value: lab_state(first)["properties"][f] == v)
        for field in BOOLEAN_PROPERTIES:
            click(prefix(first) + "Genome_" + field)
            wait("boolean property " + field, lambda _row, f=field: lab_state(first)["properties"][f] is True)
        checkpoint("lab_all_properties")
        switch_tab(first, 0)
        edit_gene(first, "GeneLength", "4.5", lambda gene: gene["length"] == 4.5)
        click(prefix(first) + "GeneLengthIncrement")
        wait("source length step", lambda _row: lab_state(first)["genes"][0]["length"] == 4.6)
        edit_gene(first, "GeneAngle", "359", lambda gene: gene["angle_degrees"] == 359)
        click(prefix(first) + "GeneAngleIncrement")
        wait("angle wraps forward", lambda _row: lab_state(first)["genes"][0]["angle_degrees"] == 0)
        click(prefix(first) + "GeneAngleDecrement")
        wait("angle wraps backward", lambda _row: lab_state(first)["genes"][0]["angle_degrees"] == 359)
        edit_gene(first, "GeneAngle", "0", lambda gene: gene["theta"] == 0)
        edit_gene(first, "GeneBranch", "0", lambda gene: gene["branch"] == 0)
        for index, channel in enumerate(STORAGE_REACTIONS):
            target = index % 5 + 1
            edit_gene(first, "Reaction_" + channel, target, lambda gene, i=index, v=target: gene["reactions"][i] == v)
        checkpoint("lab_all_reactions")
        for index, color in enumerate(colors):
            r, g, b, a = color["rgba"]
            expected_argb = a << 24 | r << 16 | g << 8 | b
            palette_choice(first, index, expected_argb, color["name"])
        palette_choice(first, 0, 0xff00ff00, "restore exportable GREEN")
        committed_state = json.loads(json.dumps(lab_state(first)))
        require(latest()["tick"] == stable_tick and latest()["paused"], "Pure laboratory edits advanced the model")
        checkpoint("lab_palette_complete")
        close_lab(first, "CommitGenome")
        wait("committed actual clipboard", lambda row: row["clipboard_ready"])
        empty = open_lab(1)
        close_lab(empty, "CommitGenome")
        copied = open_lab(0)
        require(lab_state(copied)["genes"] == committed_state["genes"] and lab_state(copied)["properties"] == committed_state["properties"], "Empty commit replaced the clipboard")
        edit_gene(copied, "GeneLength", "3", lambda gene: gene["length"] == 3)
        close_lab(copied)
        copied = open_lab(0)
        require(lab_state(copied)["genes"] == committed_state["genes"], "Cancel changed the clipboard")
        key("Escape")
        wait("laboratory Escape closes only its window", lambda row: not row["laboratories"])
        require(process.poll() is None, "Laboratory Escape quit the game")

        begin("clipboard_placement_and_corpse")
        projection_before = wait("paused projection before placement", lambda row: row["paused"] and row["world_projection_ready"])["world_projection"]
        target = empty_world_point()
        population = latest()["population"]
        menu(4)
        wait("paste placement mode", lambda row: row["placement_mode"] == "paste")
        armed_projection = wait("stable armed paste projection", lambda row: row["placement_mode"] == "paste" and row["world_projection_ready"])
        require(armed_projection["paused"] and armed_projection["world_projection"] == projection_before,
                "Arming placement changed the paused canvas projection")
        checkpoint("lab_placement_armed")
        world_point(target)
        wait("actual clipboard member insertion", lambda row: row["population"] == population + 1)
        wait("placement automatically clears", lambda row: row["placement_mode"] == "")
        cleared_projection = wait("stable projection after placement", lambda row: row["placement_mode"] == "" and row["world_projection_ready"])
        require(cleared_projection["paused"] and cleared_projection["world_projection"] == projection_before,
                "Completing placement changed the paused canvas projection")
        menu(4)
        wait("paste armed for cancellation", lambda row: row["placement_mode"] == "paste")
        rearmed_projection = wait("stable rearmed paste projection", lambda row: row["placement_mode"] == "paste" and row["world_projection_ready"])
        require(rearmed_projection["paused"] and rearmed_projection["world_projection"] == projection_before,
                "Rearming placement changed the paused canvas projection")
        key("Escape")
        wait("Escape cancels placement", lambda row: row["placement_mode"] == "" and row["population"] == population + 1)
        cancelled_projection = wait("stable projection after placement cancellation", lambda row: row["placement_mode"] == "" and row["world_projection_ready"])
        require(cancelled_projection["paused"] and cancelled_projection["world_projection"] == projection_before,
                "Cancelling placement changed the paused canvas projection")
        select_world(target)
        selected_id = latest()["selected"]["id"]
        distinct_clipboard(committed_state["genes"])
        menu(2)
        copied = open_lab(0)
        require(lab_state(copied)["genes"] == committed_state["genes"], "Copy selected genotype changed edited values")
        close_lab(copied)
        click("KillOrganismButton")
        wait("selected actual corpse", lambda row: row["selected"] and not row["selected"]["alive"])
        examined = open_lab(3)
        require(lab_state(examined)["genes"] == committed_state["genes"], "Corpse examination lost the primary genotype")
        close_lab(examined)
        distinct_clipboard(committed_state["genes"])
        observed = latest()
        native.point_at(observed, "SelectedGenomePreview")
        native.button(native.display, 3, 1, 0)
        native.button(native.display, 3, 0, 0)
        native.flush(native.display)
        wait("actual preview-copy popup", lambda row: row["preview_popup"])
        for _ in range(4):
            focused = latest()["preview_focused_id"]
            if focused == 0:
                break
            key("Down")
            wait("actual preview-copy focus", lambda row: row["preview_focused_id"] != focused)
        require(latest()["preview_focused_id"] == 0, "Preview-copy item did not receive actual focus")
        key("Return")
        wait("preview-copy popup closes", lambda row: not row["preview_popup"])
        copied = open_lab(0)
        require(lab_state(copied)["genes"] == committed_state["genes"], "Preview copy lost the corpse genotype")
        close_lab(copied)
        click("ReviveOrganismButton")
        wait("revived export selection", lambda row: row["selected"] and row["selected"]["id"] == selected_id and row["selected"]["alive"])
        checkpoint("lab_clipboard_corpse_preview")

        begin("files_and_pause_recovery")
        export = run / "laboratory-export.bgg"
        click("PauseButton")
        wait("running before export", lambda row: not row["paused"])
        menu(7)
        wait("actual export dialog", lambda row: row["file_dialog"]["visible"] and row["paused"])
        file_path(run / "laboratory-export")
        wait("export restores activity", lambda row: not row["file_operation"] and not row["paused"] and export.exists())
        proof["export_sha256"] = xml_readback(export, committed_state)
        menu(7)
        wait("second export dialog", lambda row: row["file_dialog"]["visible"])
        file_path(export)
        wait("overwrite confirmation", lambda row: row["overwrite"])
        click("OverwriteCancel")
        wait("overwrite cancel restores activity", lambda row: not row["file_operation"] and not row["paused"])
        require(hashlib.sha256(export.read_bytes()).hexdigest() == proof["export_sha256"], "Overwrite cancel changed the file")
        menu(7)
        wait("third export dialog", lambda row: row["file_dialog"]["visible"])
        file_path(export)
        wait("overwrite confirmation again", lambda row: row["overwrite"])
        click("OverwriteOkay")
        wait("overwrite confirm restores activity", lambda row: not row["file_operation"] and not row["paused"])
        require(xml_readback(export, committed_state) == proof["export_sha256"], "Confirmed export changed source values")
        invalid = run / "invalid_genotype.bgg"
        invalid.write_text("<genetic_code><gene></genetic_code>\n")
        click("FitButton")
        fitted = wait("actual Fit before malformed import", lambda row: row["fit"])
        target = [float(value) / 2 for value in fitted["world_size"]]
        target_screen = [fitted["offset"][i] + target[i] * fitted["zoom"] for i in range(2)]
        canvas_size = fitted["controls"]["WorldCanvas"][2:]
        require(all(math.isfinite(value) for value in target + target_screen) and
                all(0 <= target[i] < fitted["world_size"][i] and 5 < target_screen[i] < canvas_size[i] - 5 for i in range(2)),
                "Malformed import target must be finite, visible, and inside the world")
        menu(6)
        wait("running import placement mode", lambda row: row["placement_mode"] == "import" and not row["paused"])
        world_point(target)
        wait("actual import pauses", lambda row: row["file_dialog"]["visible"] and row["paused"])
        file_path(invalid)
        restored = wait("failed import restores activity", lambda row: not row["file_operation"] and not row["paused"] and bool(row["application_error"]))
        require(restored["application_error"].removeprefix("Error: ").startswith(("Malformed genotype XML:", "Missing XML attribute:", "Invalid XML attribute:")),
                "Owned malformed import must fail in the XML parser before placement")
        proof["failed_import_error"] = restored["application_error"]
        proof["failed_import_point"] = target
        click("PauseButton")
        wait("paused after failure recovery", lambda row: row["paused"])
        click("FitButton")
        fitted = wait("stable fitted view before independent import lab", lambda row: row["paused"] and row["fit"] and
                      not row["placement_mode"] and row["world_projection_ready"])
        canvas_width, canvas_height = fitted["controls"]["WorldCanvas"][2:]
        deselect_point = None
        for x, y in ((8, 8), (canvas_width - 8, 8), (8, canvas_height - 8), (canvas_width - 8, canvas_height - 8)):
            point = [(x - fitted["offset"][0]) / fitted["zoom"], (y - fitted["offset"][1]) / fitted["zoom"]]
            if 0 < x < canvas_width and 0 < y < canvas_height and all(math.isfinite(value) for value in point) and not (
                    0 <= point[0] < fitted["world_size"][0] and 0 <= point[1] < fitted["world_size"][1]):
                deselect_point = point
                break
        require(deselect_point is not None, "Independent import lab requires a visible letterbox deselection point")
        world_point(deselect_point)
        deselected = wait("actual deselection before independent import lab", lambda row: row["paused"] and
                          not row["selected"] and row["world_projection_ready"])
        proof["import_lab_deselection"] = {"world_point": deselect_point, "actual_pick": deselected["last_world_pick"],
                                            "selected_empty": not deselected["selected"], "projection": deselected["world_projection"]}
        imported = open_lab(1)
        click(prefix(imported) + "ImportLaboratory")
        wait("actual lab import dialog", lambda _row: laboratory(imported)["import_dialog"]["visible"])
        file_path(export, imported)
        wait("lab import complete", lambda _row: lab_state(imported)["genes"] == committed_state["genes"])
        require(lab_state(imported)["properties"] == committed_state["properties"], "Lab import metadata differs")
        switch_tab(imported, 1)
        for field, value in NUMERIC_PROPERTIES.items():
            require(latest()["texts"][prefix(imported) + "Genome_" + field] == str(value), "Imported property control is stale: " + field)
        for field in BOOLEAN_PROPERTIES:
            require(latest()["toggles"][prefix(imported) + "Genome_" + field] is True, "Imported boolean control is stale: " + field)
        checkpoint("lab_file_roundtrip")
        raw_point = run / "raw-point.bgg"
        original_bytes = export.read_bytes()
        raw_bytes, changed = re.subn(rb'(<gene\s+length=")[^"]*(")', rb'\g<1>0.0\2', original_bytes)
        require(changed == len(committed_state["genes"]) and changed > 0, "Owned point input must change every actual exported Gene length")
        original_document, point_document = ET.fromstring(original_bytes), ET.fromstring(raw_bytes)
        require(original_document.attrib == point_document.attrib, "Point input changed genotype metadata")
        for previous, point in zip(original_document.findall("gene"), point_document.findall("gene")):
            expected_attributes = dict(previous.attrib)
            expected_attributes["length"] = "0.0"
            require(point.attrib == expected_attributes, "Point input changed a non-length Gene attribute")
        with raw_point.open("xb") as owned:
            owned.write(raw_bytes)
        proof["raw_point_input"] = {"path": str(raw_point), "sha256": hashlib.sha256(raw_bytes).hexdigest(),
                                    "based_on_export_sha256": proof["export_sha256"], "changed_gene_lengths": changed}
        click(prefix(imported) + "ImportLaboratory")
        wait("actual point import dialog", lambda _row: laboratory(imported)["import_dialog"]["visible"])
        file_path(raw_point, imported)
        wait("actual raw point genes", lambda _row: bool(lab_state(imported)["genes"]) and
             all(gene["length"] == 0 for gene in lab_state(imported)["genes"]))
        point_state = lab_state(imported)
        require(len(point_state["genes"]) == len(committed_state["genes"]), "Point import changed the Gene count")
        require(point_state["properties"] == committed_state["properties"], "Point import changed metadata")
        require(point_state["editor_warnings"] and not point_state["preview_error"], "Point import lost raw warning or valid preview")
        require(point_state["preview"]["points"] and set(point_state["preview"]["points"]) == {0}, "Source point preview endpoints are not coincident at zero")
        for point, previous in zip(point_state["genes"], committed_state["genes"]):
            expected_gene = dict(previous)
            expected_gene["length"] = 0.0
            require(point == expected_gene, "Raw import changed a non-length model value")
        checkpoint("lab_raw_point")
        begin("existing_laboratory_resize")
        native.resize(native.display, main_window, 800, 660)
        native.flush(native.display)
        actions.append({"resize_existing_laboratory": [800, 660], "monotonic_ns": time.monotonic_ns()})
        resized = wait("existing laboratory layout after root resize", lambda row: row["window_size"] == [800, 660] and
                       any(lab["id"] == imported and lab["layout_ready"] for lab in row["laboratories"]))
        resized_lab = next(lab for lab in resized["laboratories"] if lab["id"] == imported)
        require(resized_lab["size"][0] >= 760 and resized_lab["size"][1] >= 620,
                "Resizing the root lost the existing laboratory's usable extent")
        require(resized_lab["state"]["genes"] == point_state["genes"] and
                resized_lab["state"]["properties"] == point_state["properties"],
                "Resizing the root changed the existing laboratory model")
        checkpoint("lab_existing_resize")
        close_lab(imported)

        begin("minimum_focus_exit")
        native.resize(native.display, main_window, 800, 660)
        native.flush(native.display)
        wait("minimum main window", lambda row: row["window_size"] == [800, 660])
        minimum = open_lab(0)
        minimum_size = laboratory(minimum)["size"]
        require(minimum_size[0] >= 760 and minimum_size[1] >= 620, "Minimum laboratory lost its declared usable extent")
        for name in ("CommitGenome", "CancelLaboratory", "ClearGenes", "ImportLaboratory"):
            visible(prefix(minimum) + name)
        checkpoint("lab_minimum")
        edit_gene(minimum, "GeneLength", "7", lambda gene: gene["length"] == 7)
        key("Escape")
        wait("focused field Escape cancels laboratory", lambda row: not row["laboratories"])
        require(process.poll() is None, "Focused laboratory Escape quit the game")
        main_focus()

        begin("random_and_import_placement")
        native.resize(native.display, main_window, 1200, 900)
        native.flush(native.display)
        actions.append({"resize_positive_placement": [1200, 900], "monotonic_ns": time.monotonic_ns()})
        wait("full main window for positive placement", lambda row: row["window_size"] == [1200, 900])
        enter("AttemptsInput", "0")
        click("CreateWorldButton")
        wait("actual replacement confirmation", lambda row: row["decision_visible"] and row["decision_controls_ready"] and row["decision"] == "create")
        checkpoint("lab_world_replace_confirmation")
        click("ConfirmWorldDecision")
        wait("actual zero-attempt world", lambda row: row["paused"] and row["tick"] == 0 and row["population"] == 0)
        click("FitButton")
        fitted = wait("stable empty world projection", lambda row: row["fit"] and row["world_projection_ready"])
        random_target = [value * 0.5 for value in fitted["world_size"]]
        menu(5)
        wait("actual random-organism placement mode", lambda row: row["placement_mode"] == "random")
        world_point(random_target)
        random_placed = wait("actual random organism insertion", lambda row: row["population"] == 1 and not row["placement_mode"])
        require(random_placed["paused"] and random_placed["tick"] == 0, "Random placement advanced biology")
        require(not random_placed["application_error"], "Random placement reported an error")
        checkpoint("lab_random_placed")

        imported_target = empty_world_point()
        menu(6)
        wait("actual positive import placement mode", lambda row: row["placement_mode"] == "import")
        world_point(imported_target)
        wait("actual positive import chooser", lambda row: row["file_dialog"]["visible"] and row["paused"])
        file_path(export)
        imported_placed = wait("actual imported organism insertion", lambda row: not row["file_operation"] and row["population"] == 2)
        require(imported_placed["paused"] and imported_placed["tick"] == 0, "Paused import advanced biology")
        require(not imported_placed["application_error"], "Positive import reported an error")
        selected_import = select_world(imported_target)
        require(selected_import["selected"]["alive"], "Imported organism is not alive")
        examined_import = open_lab(3)
        require(lab_state(examined_import)["genes"] == committed_state["genes"], "World import lost exported genes")
        require(lab_state(examined_import)["properties"] == committed_state["properties"], "World import lost exported properties")
        close_lab(examined_import)
        require(xml_readback(export, committed_state) == proof["export_sha256"], "World import modified its input")
        proof["positive_placement"] = {"random_target": random_target, "imported_target": imported_target,
                                       "imported_id": selected_import["selected"]["id"], "population": 2, "tick": 0,
                                       "input_sha256": proof["export_sha256"], "genes": len(committed_state["genes"]),
                                       "properties": len(committed_state["properties"])}
        checkpoint("lab_world_imported")
        click("QuitButton")
        wait("actual exit confirmation", lambda row: row["decision_visible"] and row["decision_controls_ready"] and row["decision"] == "quit")
        click("ConfirmWorldDecision")
        require(process.wait(timeout=5) == 0, "Actual laboratory journey exit failed")
        require(not godot_errors(log.read_text(errors="replace")), "Late laboratory frontend error")
        proof.update(result="PASS", kind="laboratory_controls", actions=len(actions), checkpoints=len(checks),
                     properties_edited=23, reactions_edited=36, palette_choices=46, model_state_injection=False,
                     ui_corrections=["Imported general controls synchronize with the imported local model"],
                     limits=["No image-export or full TASK9 acceptance", "No source callback equivalence from native GUI assertions"])
        save(run / "summary.json", proof)
        print(json.dumps({"result": "PASS", "kind": "laboratory_controls", "actions": len(actions), "checkpoints": len(checks)}), flush=True)
    except BaseException as error:
        save(run / "failure.json", {"phase": phase, "error": str(error), "actions": len(actions), "checks": len(checks)})
        raise
    finally:
        try:
            save(run / "actions.json", actions)
            save(run / "checkpoints.json", checks)
        finally:
            try:
                if native:
                    native.close(native.display)
            finally:
                try:
                    if process is not None and process.poll() is None:
                        process.terminate()
                        try:
                            process.wait(timeout=2)
                        except subprocess.TimeoutExpired:
                            process.kill()
                            process.wait(timeout=2)
                finally:
                    for number, previous in previous_handlers.items():
                        signal.signal(number, previous)
