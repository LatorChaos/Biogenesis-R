extends SceneTree

const Main = preload("res://ui/main.gd")
const MainLabels = preload("res://ui/main_labels.gd")
var _application: Control
var _checks: int = 0
var _closed: int = 0
var _output: String


func _check(value: bool, message: String) -> void:
	_checks += 1
	if not value:
		printerr("BGR_MANUAL_STARTUP_FAIL " + message)
		quit(1)
		assert(value, message)


func _frames() -> void:
	await process_frame
	await process_frame


func _press(button: Button) -> void:
	_check(is_instance_valid(button) and button.is_visible_in_tree() and not button.disabled, "Admit the actual visible button")
	button.grab_focus()
	await _frames()
	var window: Window = button.get_window()
	while window.is_embedded():
		window = window.get_parent().get_window()
	for down: bool in [true, false]:
		var event: InputEventKey = InputEventKey.new()
		event.window_id = window.get_window_id()
		event.keycode = KEY_ENTER
		event.physical_keycode = KEY_ENTER
		event.pressed = down
		Input.parse_input_event(event)
		await process_frame
	await _frames()


func _make(resume_managed: bool) -> BgrSimulation:
	var simulation: BgrSimulation = BgrSimulation.new()
	_check(bool(simulation.preferences_initialize()["ok"]), "Private preferences initialize")
	_application = Main.new()
	_application.set("startup_seed_text", "827364")
	_application.set("startup_dimensions", Vector2i(1200, 800))
	_application.set("startup_attempts", 512)
	_application.set("startup_resume_managed", resume_managed)
	_application.connect("simulation_closed", func() -> void: _closed += 1)
	_check(bool(_application.call("initialize_session", simulation)), "Main accepts the test session")
	root.add_child(_application)
	_application.set_process(false)
	return simulation


func _close() -> void:
	await _application.call("shutdown")
	_application.queue_free()
	await _frames()


func _managed(simulation: BgrSimulation) -> void:
	var draft: BgrSettingsDraft = simulation.settings_open()
	var controls: Dictionary = draft.snapshot()
	var texts: PackedStringArray = controls["texts"]
	texts[(controls["keys"] as PackedStringArray).find("BACKUP_DELAY")] = "2"
	_check(bool(draft.replace_controls({"texts": texts, "locale": controls["locale"], "automatic_backup": true, "automatic_backup_mode": 1, "hardware": controls["hardware"], "disable_framebuffer": controls["disable_framebuffer"]})["ok"]), "Managed fixture settings validate")
	_check(bool(simulation.settings_apply(draft, Callable(_application, "_settings_locale_changed"))["ok"]), "Managed fixture settings apply")


func _advance(simulation: BgrSimulation, ticks: int) -> void:
	_check(bool(simulation.apply_commands(PackedInt32Array([1]))["ok"]), "Resume fixture")
	_check(bool(simulation.advance(ticks)["ok"]), "Advance the real World")
	_application.call("_accept_result", simulation.apply_commands(PackedInt32Array([0])))


func _save(path: String) -> void:
	_application.call("_begin_file_operation", "world_save_as", Vector2.ZERO)
	_application.call("_on_genotype_file_selected", path)
	await _frames()
	var overwrite: ConfirmationDialog = _application.get("_overwrite_dialog") as ConfirmationDialog
	if overwrite.visible:
		await _press(overwrite.get_ok_button())


func _fill_quota() -> String:
	var path: String = OS.get_user_data_dir().path_join("managed-recovery/quota-unknown-test.bin")
	var file: FileAccess = FileAccess.open(path, FileAccess.WRITE)
	_check(file != null, "Open the private quota fixture")
	file.seek((1 << 30) - 1)
	file.store_8(0)
	file.close()
	return path


func _encoded(simulation: BgrSimulation, name: String) -> PackedByteArray:
	var path: String = _output.path_join(name)
	_check(bool(simulation.save_world_file(path)["ok"]), "Low-level Save keeps its ordinary contract")
	return FileAccess.get_file_as_bytes(path)


func _initialize() -> void:
	await process_frame
	_output = OS.get_environment("BGR_MANUAL_CHECK_OUTPUT")
	_check(_output.is_absolute_path() and DirAccess.dir_exists_absolute(_output), "Require private output")
	for language: String in ["ca", "de", "en", "es", "pt", "ru"]:
		for key: String in ["R_MANUAL_RECOVERY_TITLE", "R_MANUAL_RECOVERY_BODY", "R_RECOVER_OLDER", "R_SAVE_FILE_ONLY", "R_FILE_ONLY_BODY", "R_MANUAL_PENDING", "R_FILE_ONLY_WARNING"]:
			_check(MainLabels.message(key, language) != key and not MainLabels.message(key, language).is_empty(), "Localized explicit choice " + language + " " + key)
	var simulation: BgrSimulation = _make(false)
	_managed(simulation)
	_advance(simulation, 512)
	var path: String = _output.path_join("manual.bgw")
	var before: PackedByteArray = _encoded(simulation, "control-before.bgw")
	await _save(path)
	var status: Dictionary = simulation.automatic_backup_status()
	_check(not bool(status["manual_checkpoint_pending"]) and FileAccess.get_file_as_bytes(str(status["last"]["path"])) == before, "Successful manual Save publishes exact matching checkpoint")
	_check(FileAccess.get_file_as_bytes(path) == before and _encoded(simulation, "control-after.bgw") == before, "Manual transaction preserves full World and RNG bytes")
	var marker: String = OS.get_user_data_dir().path_join("managed-recovery/.biogenesis-recovery-v1")
	_check(FileAccess.get_file_as_bytes(marker).size() == 33, "Marker size and metadata quota remain fixed")
	_advance(simulation, 1)
	var newer: PackedByteArray = _encoded(simulation, "newer.bgw")
	_check(newer != before, "Real same-time tick creates distinguishable newer progress")
	var quota: String = _fill_quota()
	await _save(path)
	status = simulation.automatic_backup_status()
	_check(FileAccess.get_file_as_bytes(path) == newer and str(_application.get("_world_path")) == path, "Optional checkpoint failure preserves primary Save and adopted filename")
	_check(bool(status["manual_checkpoint_pending"]) and not str(status["manual_checkpoint_warning"]).is_empty(), "Checkpoint failure persists pending warning")
	_check(FileAccess.get_file_as_bytes(marker)[32] == 33, "Pending state is durable before Quit")
	_application.call("_quit")
	await _frames()
	await _press(_application.get("_save_world_button") as Button)
	_check(_closed == 1 and bool(_application.get("_shutting_down")), "Normal Quit still completes after successful primary Save")
	_application.queue_free()
	await _frames()
	_check(DirAccess.remove_absolute(quota) == OK, "Remove only the owned quota fault fixture")
	simulation = _make(true)
	await _frames()
	_check(not bool(_application.get("_startup_recovery_succeeded")) and bool(simulation.snapshot()["paused"]), "Pending startup refuses older autoplay")
	var choice: ConfirmationDialog = _application.get("_recovery_dialog") as ConfirmationDialog
	_check(choice.visible and bool(_application.call("_presentation_busy")), "Startup exposes a modal manual-versus-older choice")
	await _press(_application.get("_open_manual_button") as Button)
	_check(str(_application.get("_file_operation")) == "world_open", "Real choice opens the manual file dialog")
	_application.call("_on_genotype_file_selected", path)
	_check(_encoded(simulation, "manual-open.bgw") == newer and not bool(simulation.automatic_backup_status()["manual_checkpoint_pending"]), "Explicit manual Open publishes a fresh matching checkpoint before resolving pending")
	await _close()
	simulation = _make(false)
	_check(bool(simulation.open_manual_world_file(path)["ok"]), "Prepare another real manual World")
	_application.call("_accept_result", simulation.snapshot())
	_advance(simulation, 1)
	quota = _fill_quota()
	await _save(path)
	_check(bool(simulation.automatic_backup_status()["manual_checkpoint_pending"]), "Second pending intent exists")
	await _close()
	_check(DirAccess.remove_absolute(quota) == OK, "Remove the second quota fault")
	simulation = _make(true)
	await _frames()
	choice = _application.get("_recovery_dialog") as ConfirmationDialog
	await _press(choice.get_ok_button())
	_check(not bool(simulation.automatic_backup_status()["manual_checkpoint_pending"]) and bool(simulation.snapshot()["paused"]), "Explicit older checkpoint acknowledgment resolves only after valid recovery")
	_check(_encoded(simulation, "older-choice.bgw") == newer, "Older choice actually selects the prior managed bytes")
	var original: PackedByteArray = FileAccess.get_file_as_bytes(path)
	var corrupt: FileAccess = FileAccess.open(marker, FileAccess.WRITE)
	_check(corrupt != null, "Open only the private marker fault fixture")
	corrupt.store_string("invalid owned marker test fixture!")
	corrupt.close()
	_advance(simulation, 2)
	var file_only_expected: PackedByteArray = _encoded(simulation, "file-only-control.bgw")
	_check(file_only_expected != original, "File-only fixture has distinct newer progress")
	await _save(path)
	choice = _application.get("_recovery_dialog") as ConfirmationDialog
	_check(choice.visible and str(_application.get("_recovery_choice")) == "file_only" and FileAccess.get_file_as_bytes(path) == original, "Preflight failure requests acknowledgment before primary replacement")
	await _press(choice.get_cancel_button())
	_check(FileAccess.get_file_as_bytes(path) == original and str(_application.get("_file_operation")).is_empty(), "Cancel preserves manual bytes and clears the pending UI operation")
	await _save(path)
	await _press(choice.get_ok_button())
	_check(FileAccess.get_file_as_bytes(path) == file_only_expected and (_application.get("_error_label") as Label).visible, "Explicit file-only choice preserves legacy Save with a visible waiver")
	_check(FileAccess.get_file_as_bytes(marker).get_string_from_utf8() == "invalid owned marker test fixture!", "File-only choice does not repair or clear corrupt metadata")
	await _close()
	simulation = _make(true)
	await _frames()
	_check(bool(simulation.snapshot()["paused"]) and not bool(_application.get("_startup_recovery_succeeded")) and (_application.get("_recovery_dialog") as ConfirmationDialog).visible, "Corrupt metadata refuses startup and exposes explicit choices")
	await _press(_application.get("_open_manual_button") as Button)
	_application.call("_on_genotype_file_selected", path)
	_check(_encoded(simulation, "corrupt-marker-manual-open.bgw") == file_only_expected, "Corrupt recovery metadata does not prevent explicit manual Open")
	_check(bool(simulation.automatic_backup_status()["manual_checkpoint_pending"]) and not str(simulation.automatic_backup_status()["manual_checkpoint_warning"]).is_empty(), "Failed matching checkpoint cannot clear corrupt-startup warning")
	await _close()
	print("BGR_MANUAL_STARTUP " + JSON.stringify({"result": "PASS", "checks": _checks, "failures": [], "input_scope": "Engine key events for choice and normal Quit buttons; production file callbacks for fixed fixture paths"}))
	quit(0)
