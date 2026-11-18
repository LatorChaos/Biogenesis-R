extends SceneTree

const LauncherScene = preload("res://scenes/launcher.tscn")
const COMPLETE_CHECKS: Dictionary = {"bridge": 85, "ui": 49}
var _checks: int = 0
var _failures: Array[String] = []
var _observations: Array[Dictionary] = []
var _mode: String = ""
var _directory: String = ""
var _statistics_close_models: Array[Dictionary] = []

func _check(condition: bool, message: String) -> bool:
    _checks += 1
    if not condition:
        _failures.append(message)
    return condition

func _periodic(state: Dictionary) -> Array[String]:
    var shown: Dictionary = state["selected"]["display"]
    return [str(shown["energy"]), str(shown["age"]), str(shown["mass"])]

func _pick(simulation: BgrSimulation) -> Dictionary:
    var state: Dictionary = simulation.snapshot()
    var lines: PackedFloat64Array = state["segments"]
    if not _check(lines.size() >= 4, "The real World must contain rendered actors"):
        return {}
    state = simulation.select_at((lines[0] + lines[2]) / 2.0, (lines[1] + lines[3]) / 2.0, false)
    _check(bool(state["ok"]) and not state["selected"].is_empty(), "Pick a real World member")
    return state

func _step(simulation: BgrSimulation, count: int) -> Dictionary:
    var remaining: int = count
    var state: Dictionary = simulation.snapshot()
    while remaining > 0:
        var batch: int = mini(remaining, 256)
        var commands := PackedInt32Array()
        commands.resize(batch)
        commands.fill(2)
        state = simulation.apply_commands(commands)
        if not _check(bool(state["ok"]), "The bounded paused step batch must succeed"):
            return state
        remaining -= batch
    return state

func _same_world(left: Dictionary, right: Dictionary) -> bool:
    for key: String in ["width", "height", "world_time", "population", "oxygen", "carbon_dioxide", "methane", "segments", "colors", "alphas", "organism_ids"]:
        if left[key] != right[key]:
            return false
    return true

func _bridge() -> void:
    for phase: int in [5, 245, 261]:
        var simulation := BgrSimulation.new()
        _check(bool(simulation.configure(827364, 1200, 800, 16)["ok"]), "Configure the actual seeded World")
        var initial: Dictionary = _pick(simulation)
        if initial.is_empty():
            return
        _step(simulation, phase)
        simulation.track_selection()
        simulation.copy_selection()
        var saved_state: Dictionary = simulation.snapshot()
        var path: String = _directory.path_join("phase-" + str(phase) + ".bgw")
        _check(not FileAccess.file_exists(path), "Use a new owned World path")
        _check(bool(simulation.save_world_file(path)["ok"]), "Save the actual nonzero-clock World")
        var saved_bytes: PackedByteArray = FileAccess.get_file_as_bytes(path)
        _check(saved_bytes.size() > 4 and saved_bytes.slice(0, 4) == PackedByteArray([172, 237, 0, 5]), "Save uses actual Java object-stream framing")
        _step(simulation, 23)
        simulation.organism_action("feed")
        var changed: Dictionary = simulation.snapshot()
        _check(bool(simulation.statistics_open()["ok"]), "Open native Statistics before approved failed load")
        var rejected: Dictionary = simulation.open_world_file(_directory.path_join("invalid.bgw"))
        _check(not bool(rejected["statistics_open"]), "Native approved failed Open disposes Statistics before returning")
        _check(not bool(rejected["ok"]) and _same_world(changed, rejected) and rejected["tick"] == changed["tick"], "Rejected Open preserves the real World")
        _check(rejected["selected"] == changed["selected"] and rejected["tracking"] == changed["tracking"] and rejected["clipboard_ready"] == changed["clipboard_ready"], "Rejected Open preserves selected readout,tracking and clipboard")
        _check(FileAccess.get_file_as_bytes(path) == saved_bytes, "Rejected Open preserves saved bytes")

        var loaded: Dictionary = simulation.open_world_file(path)
        _check(bool(loaded["ok"]) and bool(loaded["paused"]) and _same_world(saved_state, loaded) and loaded["tick"] == changed["tick"], "Open restores persisted model/source time/gases and retains the current diagnostic tick")
        _check(loaded["selected"] == changed["selected"] and loaded["genotype_preview"] == changed["genotype_preview"] and not bool(loaded["tracking"]) and not bool(loaded["selected_is_tracked"]), "Successful replacement retains the old selected owner/readout/preview and clears tracking")
        _check(bool(loaded["clipboard_ready"]), "The copied genotype remains available across replacement")
        var selected: Dictionary = _pick(simulation)
        if selected.is_empty():
            return
        var retained: Array[String] = _periodic(selected)
        var fed: Dictionary = simulation.organism_action("feed")
        _check(float(fed["selected"]["energy"]) == float(selected["selected"]["energy"]) + minf(10.0, float(selected["carbon_dioxide"])), "Feed acts on the restored actor")
        _check(_periodic(fed) == retained, "Restored paused inspector retains its readout")
        var until_sample: int = 12 if phase == 245 else 16
        var first: Dictionary = _step(simulation, 1)
        _check(_periodic(first) == retained, "The first loaded step does not use a new-World sampling phase")
        var before_sample: Dictionary = _step(simulation, until_sample - 2)
        _check(_periodic(before_sample) == retained, "Restored sampling retains values until the source boundary")
        var sampled: Dictionary = _step(simulation, 1)
        _check(int(sampled["tick"]) == int(loaded["tick"]) + until_sample and _periodic(sampled) != retained, "Restored source boundary recalculates periodic fields")
        _check(FileAccess.get_file_as_bytes(path) == saved_bytes, "Resumed biology never rewrites the source save")
        _observations.append({"phase": phase, "loaded_tick": loaded["tick"], "loaded_source_time": loaded["world_time"], "saved_source_time": saved_state["world_time"], "preload_source_time": changed["world_time"], "sampled_tick": sampled["tick"], "retained": retained, "sampled": _periodic(sampled), "bytes": saved_bytes.size()})

func _pick_main(main: Control, simulation: BgrSimulation) -> Dictionary:
    var state: Dictionary = simulation.snapshot()
    var lines: PackedFloat64Array = state["segments"]
    (main.get("_canvas") as Control).emit_signal("organism_picked", Vector2(floor(lines[0]), floor(lines[1])), false, Vector2.ZERO)
    var selected: Dictionary = simulation.snapshot()
    _check(not selected["selected"].is_empty(), "Canvas selection reaches the actual restored World")
    return selected

func _pick_other_main(main: Control, simulation: BgrSimulation, excluded_id: int) -> Dictionary:
    var state: Dictionary = simulation.snapshot()
    var lines: PackedFloat64Array = state["segments"]
    var identities: PackedInt64Array = state["organism_ids"]
    for index: int in range(mini(identities.size(), 256)):
        if identities[index] == excluded_id:
            continue
        var offset: int = index * 4
        (main.get("_canvas") as Control).emit_signal("organism_picked", Vector2(floor((lines[offset] + lines[offset + 2]) / 2.0), floor((lines[offset + 1] + lines[offset + 3]) / 2.0)), false, Vector2.ZERO)
        var selected: Dictionary = simulation.snapshot()
        if not selected["selected"].is_empty() and int(selected["selected"]["id"]) != excluded_id:
            return selected
    return simulation.snapshot()

func _laboratory(main: Control) -> Window:
    for child: Node in main.get_children():
        if child.get_script() == preload("res://ui/laboratory_window.gd"):
            return child as Window
    return null

func _record_statistics_close(simulation: BgrSimulation) -> void:
    _statistics_close_models.append(simulation.snapshot())

func _choose_file(main: Control, command: int, path: String) -> void:
    main.call("_on_world_action", command)
    var dialog: FileDialog = main.get("_file_dialog") as FileDialog
    _check(not str(main.get("_file_operation")).is_empty(), "Actual World action acquires the FileDialog")
    if command == 11 and not str(main.get("_world_path")).is_empty():
        _check(dialog.current_file.is_empty(), "Open requires a fresh selection instead of a stale filename preset")
    dialog.file_selected.emit(path)
    _check(str(main.get("_file_operation")).is_empty(), "FileDialog completion releases the operation")

func _fixture_locale_changed(payload: Dictionary) -> void:
    _observations.append({"fixture_language": str(payload["language"])})

func _prepare_owned_profile() -> bool:
    var path: String = OS.get_user_data_dir().path_join("biogenesis-r.preferences.xml")
    var setup: BgrSimulation = BgrSimulation.new()
    if not _check(not FileAccess.file_exists(path) and bool(setup.preferences_initialize()["ok"]) and not bool(setup.preferences_snapshot()["session_ready"]), "Initialize fresh owned preferences before fixture genesis"):
        return false
    var token: BgrSettingsDraft = setup.settings_open()
    var state: Dictionary = token.snapshot()
    var keys: PackedStringArray = state["keys"]
    var texts: PackedStringArray = state["texts"]
    if not _check(keys.has("WORLD_WIDTH") and keys.has("WORLD_HEIGHT") and keys.has("INITIAL_ORGANISMS") and keys.size() == texts.size(), "Declared typed settings expose all bounded fixture genesis controls"):
        return false
    var changes: Dictionary = {"WORLD_WIDTH": "1200", "WORLD_HEIGHT": "800", "INITIAL_ORGANISMS": "32"}
    for key: String in changes:
        texts[keys.find(key)] = str(changes[key])
    var controls: Dictionary = {"texts": texts, "locale": state["locale"], "automatic_backup": state["automatic_backup"], "hardware": state["hardware"], "disable_framebuffer": state["disable_framebuffer"]}
    if not _check(bool(token.replace_controls(controls)["ok"]) and bool(setup.settings_apply(token, _fixture_locale_changed)["ok"]) and setup.settings_snapshot()["genesis"] == PackedInt64Array([1200, 800, 32]), "Apply bounded fixture genesis through the public typed settings boundary"):
        return false
    var geometry: PackedInt32Array = setup.preferences_snapshot()["geometry"]
    var saved: Dictionary = setup.preferences_save(geometry)
    if not _check(bool(saved["ok"]) and bool(saved["saved"]) and FileAccess.file_exists(path) and not bool(setup.preferences_snapshot()["session_ready"]), "Persist fixture settings without creating a World or session"):
        return false
    return true

func _ui() -> void:
    if not _prepare_owned_profile():
        return
    var launcher: Control = LauncherScene.instantiate() as Control
    root.add_child(launcher)
    await process_frame
    await process_frame
    (launcher.get("_start_stop") as Button).pressed.emit()
    await process_frame
    await process_frame
    var main: Control = launcher.get("_main") as Control
    if not _check(is_instance_valid(main), "Launch the actual Main window"):
        return
    var simulation: BgrSimulation = main.get("_simulation") as BgrSimulation
    var initialized: Dictionary = simulation.snapshot()
    if not _check(int(initialized["width"]) == 1200 and int(initialized["height"]) == 800 and int(initialized["initial_attempts"]) == 32 and int(initialized["tick"]) == 0 and bool(initialized["paused"]), "Launcher consumes the persisted bounded fixture genesis before any steps"):
        main.call("shutdown")
        launcher.queue_free()
        return
    var panel: Control = main.get("_organism_controls") as Control
    var first_selected: Dictionary = _pick_main(main, simulation)
    panel.emit_signal("preview_copy_requested")
    _check(bool(simulation.snapshot()["clipboard_ready"]), "Preview Copy establishes the original clipboard through Main")
    var other_selected: Dictionary = _pick_other_main(main, simulation, int(first_selected["selected"]["id"]))
    if not _check(not other_selected["selected"].is_empty() and other_selected["selected"]["id"] != first_selected["selected"]["id"], "Choose a distinct real selected organism while retaining the earlier clipboard"):
        main.call("shutdown")
        launcher.queue_free()
        return
    _check(bool(main.call("_accept_result", simulation.track_selection())), "Track the actual second selection through the existing native boundary")
    main.call("_open_laboratory", "clipboard")
    await process_frame
    var laboratory: Window = _laboratory(main)
    if not _check(is_instance_valid(laboratory) and not (laboratory.get("_state") as Dictionary)["genes"].is_empty(), "Open a nonempty cloned clipboard Laboratory"):
        main.call("shutdown")
        launcher.queue_free()
        return
    var length_input: LineEdit = laboratory.find_child("GeneLength", true, false) as LineEdit
    var old_length: float = float((laboratory.get("_state") as Dictionary)["genes"][0]["length"])
    length_input.text = "2" if old_length != 2.0 else "3"
    length_input.text_submitted.emit(length_input.text)
    var laboratory_state: Dictionary = (laboratory.get("_state") as Dictionary).duplicate(true)
    var laboratory_draft: Object = laboratory.get("draft")
    _check(float(laboratory_state["genes"][0]["length"]) == float(length_input.text) and float(laboratory_state["genes"][0]["length"]) != old_length, "Edit only the Laboratory clone through its existing field control")
    for index: int in range(5):
        (main.get("_step_button") as Button).pressed.emit()
    var saved: Dictionary = simulation.snapshot()
    var path: String = _directory.path_join("main-phase5.bgw")
    _choose_file(main, 12, path)
    _check(FileAccess.file_exists(path) and str(main.get("_world_path")) == path and int(saved["tick"]) == 5, "Actual Save owns the phase-five file")
    var saved_bytes: PackedByteArray = FileAccess.get_file_as_bytes(path)
    main.call("_on_world_action", 13)
    var save_as: FileDialog = main.get("_file_dialog") as FileDialog
    _check(save_as.current_file == path.get_file() and save_as.current_dir == path.get_base_dir(), "Save As retains the current directory and basename")
    save_as.canceled.emit()
    _check(str(main.get("_file_operation")).is_empty() and FileAccess.get_file_as_bytes(path) == saved_bytes, "Cancel Save As releases the operation and preserves file bytes")
    var buttons: Dictionary = panel.get("_buttons")
    (buttons["feed"] as Button).pressed.emit()
    var before_bad: Dictionary = simulation.snapshot()
    var before_text: String = (panel.get("_summary") as Label).text + (panel.get("_lineage") as Label).text
    main.call("_open_statistics")
    await process_frame
    var old_statistics: Window = main.get("_statistics_window") as Window
    _check(is_instance_valid(old_statistics), "Open the actual statistics owner")
    old_statistics.connect("closed", _record_statistics_close.bind(simulation))
    main.call("_toggle_pause")
    main.call("_on_world_action", 11)
    (main.get("_file_dialog") as FileDialog).canceled.emit()
    _check(main.get("_statistics_window") == old_statistics and bool(simulation.snapshot()["statistics_open"]) and simulation.snapshot()["selected"] == before_bad["selected"] and bool(simulation.snapshot()["tracking"]) and bool(simulation.snapshot()["clipboard_ready"]) and bool(simulation.snapshot()["paused"]), "Chooser Cancel keeps Statistics, selected owner, tracking, clipboard and paused state")
    _check(is_instance_valid(laboratory) and laboratory.get("draft") == laboratory_draft and (laboratory.get("_state") as Dictionary)["genes"] == laboratory_state["genes"], "Chooser Cancel retains the same Laboratory draft and genes")
    _choose_file(main, 11, _directory.path_join("invalid.bgw"))
    await process_frame
    _check(simulation.snapshot()["selected"] == before_bad["selected"] and panel.visible, "Failed UI Open preserves selected data and panel visibility")
    _check((panel.get("_summary") as Label).text + (panel.get("_lineage") as Label).text == before_text, "Failed UI Open preserves cached labels")
    _check(not is_instance_valid(old_statistics) and not bool(simulation.snapshot()["statistics_open"]) and (main.get("_error_label") as Label).visible, "Approved failed UI Open closes Statistics and reports failure")
    _check(_statistics_close_models.size() == 1 and _same_world(_statistics_close_models[0], before_bad), "Failed Open closes Statistics against the unchanged old model")
    _check(is_instance_valid(laboratory) and laboratory.get("draft") == laboratory_draft and (laboratory.get("_state") as Dictionary)["genes"] == laboratory_state["genes"], "Approved failed Open retains the same Laboratory draft")
    main.call("_open_statistics")
    await process_frame
    var success_statistics: Window = main.get("_statistics_window") as Window
    _check(is_instance_valid(success_statistics), "Reopen Statistics before successful replacement")
    success_statistics.connect("closed", _record_statistics_close.bind(simulation))
    var before_success: Dictionary = simulation.snapshot()

    _choose_file(main, 11, path)
    await process_frame
    await process_frame
    var loaded: Dictionary = simulation.snapshot()
    _check(_same_world(saved, loaded) and loaded["tick"] == before_bad["tick"] and loaded["selected"] == before_success["selected"] and loaded["genotype_preview"] == before_success["genotype_preview"] and panel.visible and not bool(loaded["tracking"]) and bool(loaded["clipboard_ready"]) and not bool(panel.call("has_open_popup")), "Successful UI Open restores the new model and retains old inspection, primary preview and clipboard")
    _check(not is_instance_valid(success_statistics) and not is_instance_valid(main.get("_statistics_window")) and _statistics_close_models.size() == 2 and _same_world(_statistics_close_models[1], before_success), "Successful approval releases Statistics before publishing the replacement model")
    if not _check(is_instance_valid(laboratory) and laboratory.get("draft") == laboratory_draft and (laboratory.get("_state") as Dictionary)["genes"] == laboratory_state["genes"], "Successful Open preserves the original nonmodal Laboratory and edited clone"):
        main.call("shutdown")
        launcher.queue_free()
        return
    laboratory.call("_commit")
    await process_frame
    _check(not is_instance_valid(laboratory) and _same_world(saved, simulation.snapshot()) and bool(simulation.snapshot()["clipboard_ready"]), "Surviving Laboratory commits through the same Main simulation and current loaded World response")
    main.call("_open_laboratory", "clipboard")
    await process_frame
    var committed_laboratory: Window = _laboratory(main)
    if not _check(is_instance_valid(committed_laboratory) and (committed_laboratory.get("_state") as Dictionary)["genes"] == laboratory_state["genes"] and (committed_laboratory.get("_state") as Dictionary)["properties"] == laboratory_state["properties"], "Current application clipboard receives the surviving edited clone rather than the old selected genotype"):
        main.call("shutdown")
        launcher.queue_free()
        return
    committed_laboratory.call("_cancel")
    await process_frame
    var selected: Dictionary = _pick_main(main, simulation)
    var retained: Array[String] = _periodic(selected)
    _check(panel.visible, "Picking a restored actor shows inspection controls")
    (buttons["feed"] as Button).pressed.emit()
    _check(_periodic(simulation.snapshot()) == retained, "Actual Feed preserves the loaded paused readout")
    for index: int in range(15):
        (main.get("_step_button") as Button).pressed.emit()
    _check(_periodic(simulation.snapshot()) == retained, "Main retains restored values through source frame20")
    (main.get("_step_button") as Button).pressed.emit()
    var sampled: Dictionary = simulation.snapshot()
    _check(int(sampled["tick"]) == 21 and _periodic(sampled) != retained, "Actual Step reaches the restored source frame21 boundary")
    _check(str(sampled["selected"]["display"]["energy"]) in (panel.get("_summary") as Label).text, "Recalculated energy reaches visible Main labels")
    main.call("_open_statistics")
    await process_frame
    var new_statistics: Window = main.get("_statistics_window") as Window
    _check(is_instance_valid(new_statistics), "A fresh statistics owner opens after replacement")
    (new_statistics.get("_update_button") as Button).pressed.emit()
    _check(FileAccess.get_file_as_bytes(path) == saved_bytes, "Inspection and statistics do not rewrite the loaded file")
    _observations.append({"UI_saved_tick": saved["tick"], "UI_sampled_tick": sampled["tick"], "old_statistics_released": not is_instance_valid(old_statistics), "new_statistics_open": is_instance_valid(new_statistics)})
    (launcher.get("_start_stop") as Button).pressed.emit()
    await process_frame
    _check(not is_instance_valid(launcher.get("_main")), "Stop releases the restored Main and its children")
    launcher.queue_free()
    await process_frame

func _initialize() -> void:
    await process_frame
    _mode = OS.get_environment("BGR_INSPECTOR_MODE")
    _directory = OS.get_environment("BGR_GUI_OUTPUT")
    if not _check(_directory.is_absolute_path() and DirAccess.dir_exists_absolute(_directory), "Use an existing owned output directory"):
        _finish()
        return
    var owned_profile: String = OS.get_environment("BGR_TEST_ROOT")
    if not _check(owned_profile.is_absolute_path() and OS.get_user_data_dir().begins_with(owned_profile + "/"), "Use an owned isolated user-data profile before constructing Launcher"):
        _finish()
        return
    var bad := FileAccess.open(_directory.path_join("invalid.bgw"), FileAccess.WRITE)
    if not _check(bad != null, "Create the owned malformed-file boundary fixture"):
        _finish()
        return
    bad.store_buffer(PackedByteArray([0, 1, 2, 3]))
    bad.close()
    if _mode == "bridge":
        _bridge()
    elif _mode == "ui":
        await _ui()
    else:
        _check(false, "Unknown restore probe mode")
    _finish()

func _finish() -> void:
    if _failures.is_empty() and _checks != int(COMPLETE_CHECKS.get(_mode, -1)):
        _failures.append("Complete checker count differs from the frozen mode denominator")
    print("BGR_SELECTED_RESTORE_SUMMARY " + JSON.stringify({"status": "PASS" if _failures.is_empty() else "FAIL", "mode": _mode, "checks": _checks, "failures": _failures, "observations": _observations, "native_input": false, "Core_state_injection": false, "real_control_journey": false, "complete_checks_expected": COMPLETE_CHECKS.get(_mode, -1), "scope": "Direct native and UI-signal seams only"}))
    quit(0 if _failures.is_empty() else 1)
