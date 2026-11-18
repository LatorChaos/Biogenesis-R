extends SceneTree

const MainLabels = preload("res://ui/main_labels.gd")

const Main = preload("res://ui/main.gd")

var _application: Control
var _checks: int = 0


func _initialize() -> void:
	await process_frame
	var directory: String = OS.get_environment("BGR_MANUAL_BACKUP_OUTPUT")
	if not _check(directory.is_absolute_path() and DirAccess.dir_exists_absolute(directory), "An existing private absolute output directory is required"):
		return
	for name: String in ["manual.bgw", "manual@00000.bgw", "manual@00001.bgw", "manual-failure.bgw", "manual-failure@00000.bgw"]:
		if not _check(not FileAccess.file_exists(directory.path_join(name)) and not DirAccess.dir_exists_absolute(directory.path_join(name)), "Owned scenario paths must initially be absent"):
			return
	var simulation: BgrSimulation = BgrSimulation.new()
	if not _check(simulation.has_method("backup_world_file"), "The actual native library must register manual Backup"):
		return
	_application = Main.new()
	_application.set("startup_seed_text", "827364")
	_application.set("startup_dimensions", Vector2i(1200, 800))
	_application.set("startup_attempts", 0)
	if not _check(bool(_application.call("initialize_session", simulation)), "The application must accept its actual session"):
		return
	root.add_child(_application)
	var initial: Dictionary = simulation.snapshot()
	if not _check(bool(initial["ok"]) and initial.has("world_time") and int(initial["world_time"]) == 0, "The batch snapshot must expose initial Source game time"):
		return
	_application.call("_on_world_action", 14)
	if not _check(str(_application.get("_file_operation")) == "world_save_as", "Unnamed Backup must use the existing Save As flow"):
		return
	_application.call("_finish_file_operation")
	if not _check(str(_application.get("_world_path")).is_empty() and _same(initial, simulation.snapshot()), "Cancelling unnamed Backup must retain the unnamed paused world"):
		return
	var path: String = directory.path_join("manual.bgw")
	_application.call("_on_world_action", 14)
	_application.call("_on_genotype_file_selected", path.trim_suffix(".bgw"))
	if not _check(str(_application.get("_world_path")) == path and FileAccess.file_exists(path) and not FileAccess.file_exists(directory.path_join("manual@00000.bgw")), "Successful unnamed Backup must save and adopt the chosen ordinary world filename"):
		return
	var active_bytes: PackedByteArray = FileAccess.get_file_as_bytes(path)
	_application.call("_toggle_pause")
	if not _check(bool(_application.call("_accept_result", simulation.advance(255))), "Actual running advancement must succeed"):
		return
	var running: Dictionary = simulation.snapshot()
	if not _check(int(running["tick"]) == 255 and int(running["world_time"]) == 0 and not bool(running["paused"]), "Source game time must remain zero before frame 256"):
		return
	_application.call("_on_world_action", 14)
	var backup_zero: String = directory.path_join("manual@00000.bgw")
	if not _check(FileAccess.file_exists(backup_zero) and str(_application.get("_world_path")) == path and _same(running, simulation.snapshot()), "Named Backup must use game time and preserve the running world and current filename"):
		return
	var first: PackedByteArray = FileAccess.get_file_as_bytes(backup_zero)
	_application.call("_on_world_action", 6)
	var changed: Dictionary = simulation.snapshot()
	_application.call("_on_world_action", 14)
	if not _check(FileAccess.get_file_as_bytes(backup_zero) != first and not (_application.get("_overwrite_dialog") as ConfirmationDialog).visible and _same(changed, simulation.snapshot()) and FileAccess.get_file_as_bytes(path) == active_bytes, "Same-time Backup must replace its backup without replacing the active file"):
		return
	if not _check(bool(_application.call("_accept_result", simulation.advance(1))), "The actual 256th frame must succeed"):
		return
	var one: Dictionary = simulation.snapshot()
	_application.call("_on_world_action", 14)
	var backup_one: String = directory.path_join("manual@00001.bgw")
	if not _check(int(one["tick"]) == 256 and int(one["world_time"]) == 1 and FileAccess.file_exists(backup_one) and _same(one, simulation.snapshot()), "Backup names must use the incremented Source time instead of Rust tick"):
		return
	_application.call("_toggle_pause")
	var paused: Dictionary = simulation.snapshot()
	_application.call("_on_world_action", 14)
	if not _check(_same(paused, simulation.snapshot()) and str(_application.get("_world_path")) == path, "Named Backup must preserve an already paused world"):
		return
	_application.call("_on_world_action", 11)
	_application.call("_on_genotype_file_selected", backup_one)
	var loaded: Dictionary = simulation.snapshot()
	var loaded_bytes: PackedByteArray = FileAccess.get_file_as_bytes(backup_one)
	_application.call("_on_world_action", 14)
	if not _check((_application.get("_error_label") as Label).visible and str(_application.get("_world_path")) == backup_one and _same(loaded, simulation.snapshot()) and FileAccess.get_file_as_bytes(backup_one) == loaded_bytes, "Backup of the active same-time timestamped file must fail visibly without changing it"):
		return

	_application.call("_on_world_action", 11)
	_application.call("_on_genotype_file_selected", path)
	var failure_path: String = directory.path_join("manual-failure.bgw")
	var failure_target: String = directory.path_join("manual-failure@00000.bgw")
	_application.call("_on_world_action", 13)
	_application.call("_on_genotype_file_selected", failure_path)
	if not _check(DirAccess.make_dir_absolute(failure_target) == OK, "Cannot create the owned destination failure"):
		return
	for running_state: bool in [false, true]:
		if bool(simulation.snapshot()["paused"]) == running_state:
			_application.call("_toggle_pause")
		var before_failure: Dictionary = simulation.snapshot()
		_application.call("_on_world_action", 14)
		if not _check((_application.get("_error_label") as Label).visible and str(_application.get("_world_path")) == failure_path and str(_application.get("_file_operation")).is_empty() and _same(before_failure, simulation.snapshot()) and DirAccess.dir_exists_absolute(failure_target), "Backup writer failure must retain activity, filename, world, and destination"):
			return
	_application.call("_toggle_pause")
	if not _check(DirAccess.remove_absolute(failure_target) == OK, "Cannot remove the owned empty failure directory"):
		return

	var languages: Array[String] = ["en", "es", "ca", "de", "pt", "ru"]
	var keys: Array[int] = [KEY_B, KEY_R, KEY_G, KEY_K, KEY_B, KEY_B]
	for index: int in range(languages.size()):
		var draft: BgrSettingsDraft = simulation.settings_open()
		var controls: Dictionary = draft.snapshot()
		if not _check(bool(draft.replace_controls({"texts": controls["texts"], "locale": index, "automatic_backup": controls["automatic_backup"], "hardware": controls["hardware"], "disable_framebuffer": controls["disable_framebuffer"]})["ok"]), "Locale controls must be accepted"):
			return
		if not _check(bool(simulation.settings_apply(draft, Callable(_application, "_settings_locale_changed"))["ok"]), "Actual settings application must update the Backup accelerator"):
			return
		if FileAccess.file_exists(failure_target) and not _check(DirAccess.remove_absolute(failure_target) == OK, "Cannot remove the prior owned shortcut backup"):
			return
		_application.get_viewport().gui_release_focus()
		var event: InputEventKey = InputEventKey.new()
		event.pressed = true
		event.physical_keycode = keys[index]
		event.keycode = keys[index]
		var before_key: Dictionary = simulation.snapshot()
		_application.call("_input", event)
		if not _check(FileAccess.file_exists(failure_target) and str(_application.get("_decision_operation")).is_empty() and _same(before_key, simulation.snapshot()) and str(_application.get("_world_path")) == failure_path, "Source Backup accelerator must reach the actual writer: " + languages[index]):
			return
		var reset: Button = _application.get("_reset_button") as Button
		var help: Label = _application.get("_help_label") as Label
		if languages[index] == "es":
			event.ctrl_pressed = true
			_application.call("_input", event)
			if not _check(str(_application.get("_decision_operation")) == "reset" and "Ctrl+R" in reset.text and "Ctrl+R" in help.text, "Spanish Ctrl+R must retain Reset with accurate shortcut labels"):
				return
			_application.call("_cancel_world_decision")
		elif not _check(reset.text == MainLabels.message("R_RESET", languages[index]) + " [R]", "Nonconflicting locales must retain the existing Reset shortcut"):
			return
	_application.call("shutdown")
	_application.queue_free()
	print("BGR_MANUAL_BACKUP_BOUNDARY_PASS " + str(_checks))
	quit(0)


func _same(left: Dictionary, right: Dictionary) -> bool:
	for key: String in ["seed", "tick", "world_time", "paused", "width", "height", "population", "oxygen", "carbon_dioxide", "methane", "segments", "colors", "alphas", "organism_ids", "selected", "tracking", "clipboard_ready"]:
		if left.get(key) != right.get(key):
			return false
	return true


func _check(value: bool, message: String) -> bool:
	_checks += 1
	if not value:
		printerr("BGR_MANUAL_BACKUP_BOUNDARY_FAIL " + message)
		if is_instance_valid(_application):
			_application.call("shutdown")
		quit(1)
	return value
