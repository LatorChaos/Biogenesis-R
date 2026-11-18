extends SceneTree

var _checks: int = 0
var _locale_callbacks: int = 0


func _check(value: bool, message: String) -> bool:
	_checks += 1
	if not value:
		printerr("BGR_BACKGROUND_RECOVERY_FAIL " + message)
		quit(1)
		return false
	return true


func _locale_changed(_value: Dictionary) -> void:
	_locale_callbacks += 1


func _initialize() -> void:
	await process_frame
	var directory: String = OS.get_environment("BGR_BACKGROUND_CHECK_OUTPUT")
	if not _check(directory.is_absolute_path() and DirAccess.dir_exists_absolute(directory), "Private output must exist"):
		return
	var simulation: BgrSimulation = BgrSimulation.new()
	if not _check(bool(simulation.preferences_initialize()["ok"]), "Preferences initialize"):
		return
	if not _check(bool(simulation.configure(827364, 1200, 800, 32)["ok"]), "World configures"):
		return
	if not _check(int(simulation.snapshot()["population"]) > 0, "World contains organisms"):
		return
	simulation.configure_automatic_backups(OS.get_user_data_dir(), "")
	var draft: BgrSettingsDraft = simulation.settings_open()
	var controls: Dictionary = draft.snapshot()
	var texts: PackedStringArray = controls["texts"]
	texts[(controls["keys"] as PackedStringArray).find("BACKUP_DELAY")] = "1"
	if not _check(bool(draft.replace_controls({"texts": texts, "locale": controls["locale"], "automatic_backup": true, "automatic_backup_mode": 1, "hardware": controls["hardware"], "disable_framebuffer": controls["disable_framebuffer"]})["ok"]), "Managed settings validate"):
		return
	if not _check(bool(simulation.settings_apply(draft, Callable(self, "_locale_changed"))["ok"]), "Managed settings apply"):
		return
	if not _check(bool(simulation.apply_commands(PackedInt32Array([1]))["ok"]), "World resumes"):
		return
	while int(simulation.snapshot()["tick"]) < 256:
		if not _check(bool(simulation.advance_budgeted(256 - int(simulation.snapshot()["tick"]), 50000)["ok"]), "Budgeted advancement succeeds"):
			return
	var status: Dictionary = simulation.automatic_backup_status()
	if not _check(int(status["attempts"]) == 1, "The first exact boundary starts one checkpoint"):
		return
	var expected_path: String = directory.path_join("captured.bgw")
	if not _check(bool(simulation.save_world_file(expected_path)["ok"]), "Save waits for any in-flight checkpoint"):
		return
	status = simulation.automatic_backup_status()
	if not _check(not bool(status["checkpoint_pending"]) and str(status["outcome"]) == "saved", "Finished checkpoint is reported"):
		return
	var checkpoint_path: String = str((status["last"] as Dictionary)["path"])
	if not _check(int((status["last"] as Dictionary)["time"]) == 1, "Checkpoint reports its captured time"):
		return
	if not _check(FileAccess.get_file_as_bytes(checkpoint_path) == FileAccess.get_file_as_bytes(expected_path), "Checkpoint exactly matches the captured world"):
		return
	if not _check(bool(simulation.apply_commands(PackedInt32Array([6]))["ok"]), "World can change after capture"):
		return
	var recovered: Dictionary = simulation.recover_latest_world()
	if not _check(bool(recovered["ok"]) and bool(recovered["paused"]) and int(recovered["world_time"]) == 1, "Recovery publishes the captured paused world"):
		return
	var restored_path: String = directory.path_join("restored.bgw")
	if not _check(bool(simulation.save_world_file(restored_path)["ok"]), "Recovered world saves"):
		return
	if not _check(FileAccess.get_file_as_bytes(checkpoint_path) == FileAccess.get_file_as_bytes(restored_path), "Recovered world matches checkpoint bytes"):
		return
	if not _check(bool(simulation.apply_commands(PackedInt32Array([1]))["ok"]), "Recovered world resumes"):
		return
	while int(simulation.snapshot()["world_time"]) < 2:
		if not _check(bool(simulation.advance_budgeted(64, 50000)["ok"]), "Advance to the next managed boundary"):
			return
	var manual_path: String = directory.path_join("manual.bgw")
	var manual: Dictionary = simulation.save_manual_world_file(manual_path, false)
	if not _check(bool(manual["ok"]) and not bool(manual["manual_checkpoint_pending"]), "Manual Save completes its recovery interlock"):
		return
	status = simulation.automatic_backup_status()
	if not _check(not bool(status["checkpoint_pending"]) and int((status["last"] as Dictionary)["time"]) == 2, "Manual Save follows the pending automatic checkpoint"):
		return
	if not _check(FileAccess.get_file_as_bytes(manual_path) == FileAccess.get_file_as_bytes(str((status["last"] as Dictionary)["path"])), "Manual Save and its recovery point match"):
		return
	print("BGR_BACKGROUND_RECOVERY_PASS " + str(_checks))
	quit(0)
