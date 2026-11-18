extends SceneTree

const Main = preload("res://ui/main.gd")

var _application: Control
var _checks: int = 0


func _initialize() -> void:
	await process_frame
	var directory: String = OS.get_environment("BGR_AUTOMATIC_BACKUP_OUTPUT")
	if not _check(directory.is_absolute_path() and DirAccess.dir_exists_absolute(directory), "An existing private absolute output directory is required"):
		return
	var simulation: BgrSimulation = BgrSimulation.new()
	if not _check(simulation.has_method("configure_automatic_backups") and simulation.has_method("recover_latest_world") and simulation.has_method("complete_automatic_backup"), "The actual host must register automatic persistence methods"):
		return
	if not _check(bool(simulation.preferences_initialize()["ok"]), "Owned preferences must initialize"):
		return
	_make_application(simulation)
	if not _check(int(simulation.snapshot()["world_time"]) == 0 and not DirAccess.dir_exists_absolute(OS.get_user_data_dir().path_join("managed-recovery")), "Startup must not recover or write a snapshot"):
		return
	_application.call("_toggle_pause")
	var before_empty: Dictionary = simulation.snapshot()
	var before_file: String = directory.path_join("before-empty-recovery.bgw")
	var after_file: String = directory.path_join("after-empty-recovery.bgw")
	if not _check(bool(simulation.save_world_file(before_file)["ok"]), "The failed-recovery comparison fixture must save"):
		return
	_application.set("_world_path", before_file)
	_application.call("_on_world_action", 15)
	var after_empty: Dictionary = simulation.snapshot()
	if not _check(_same_model(before_empty, after_empty) and not bool(after_empty["paused"]) and str(_application.get("_world_path")) == before_file and (_application.get("_error_label") as Label).visible and bool(simulation.save_world_file(after_file)["ok"]) and FileAccess.get_file_as_bytes(before_file) == FileAccess.get_file_as_bytes(after_file), "Failed recovery must retain the running world, encoded owner graph, path, and visible error"):
		return
	var continued: Dictionary = simulation.advance(1)
	if not _check(bool(continued["ok"]) and int(continued["tick"]) == int(before_empty["tick"]) + 1, "The running world must continue after failed recovery"):
		return
	_application.call("_accept_result", continued)
	_application.call("_toggle_pause")
	_application.call("_open_settings")
	var window: Window = _application.get("_settings_window") as Window
	(window.get("_backup_mode") as OptionButton).select(1)
	(window.get("_backup") as CheckBox).set_pressed_no_signal(true)
	window.call("_cancel_draft")
	await process_frame
	if not _check(int(simulation.settings_snapshot()["automatic_backup_mode"]) == 0 and not bool(simulation.settings_snapshot()["values"]["AUTO_BACKUP"]), "Cancelling actual mode controls must retain committed settings"):
		return
	if not (await _settings(simulation, 1, true, "2")):
		return
	if not _check(int(simulation.settings_snapshot()["automatic_backup_mode"]) == 1 and str(simulation.automatic_backup_status()["mode"]) == "managed", "Actual settings controls must apply the authoritative native mode"):
		return
	var pending_status: Dictionary = simulation.automatic_backup_status()
	pending_status["checkpoint_pending"] = true
	pending_status["enabled"] = false
	_application.call("_update_automatic_status", pending_status)
	if not _check((_application.get("_automatic_label") as Label).visible and (_application.get("_automatic_label") as Label).text.contains("Saving recovery point."), "A pending checkpoint must remain visible after disabling backups"):
		return
	_application.call("_process", 0.0)
	if not _check(not (_application.get("_automatic_label") as Label).text.contains("Saving recovery point."), "A completed checkpoint must clear while paused"):
		return
	var compatibility: BgrSettingsDraft = simulation.settings_open()
	var original_controls: Dictionary = compatibility.snapshot()
	var old_controls: Dictionary = {"texts": original_controls["texts"], "locale": original_controls["locale"], "automatic_backup": original_controls["automatic_backup"], "hardware": original_controls["hardware"], "disable_framebuffer": original_controls["disable_framebuffer"]}
	var invalid_mode: Dictionary = old_controls.duplicate(true)
	invalid_mode["automatic_backup_mode"] = 9
	if not _check(not bool(compatibility.replace_controls(invalid_mode)["ok"]) and compatibility.snapshot() == original_controls, "Invalid native mode must reject the entire control batch atomically"):
		return
	if not _check(bool(compatibility.replace_controls(old_controls)["ok"]) and int(compatibility.snapshot()["automatic_backup_mode"]) == 1 and bool(compatibility.cancel()["ok"]), "Existing five-group controls must preserve the staged native mode"):
		return
	_application.call("_toggle_pause")
	var advanced: Dictionary = simulation.advance(1024)
	_application.call("_accept_result", advanced)
	var status: Dictionary = simulation.automatic_backup_status()
	var last: Dictionary = status["last"]
	var managed_path: String = str(last.get("path", ""))
	if not _check(bool(advanced["ok"]) and int(advanced["world_time"]) == 4 and int(status["attempts"]) == 2 and int(last.get("time", -1)) == 4 and FileAccess.file_exists(managed_path) and not bool(advanced["paused"]), "Bulk advancement must save both exact managed boundaries without pausing"):
		return
	var managed_bytes: PackedByteArray = FileAccess.get_file_as_bytes(managed_path)
	for _poll: int in range(3):
		simulation.snapshot()
		simulation.automatic_backup_status()
	if not _check(simulation.automatic_backup_status() == status and FileAccess.get_file_as_bytes(managed_path) == managed_bytes, "Snapshot and status polling must remain side-effect free"):
		return
	_application.call("_toggle_pause")
	_application.call("_on_world_action", 11)
	_application.call("_on_genotype_file_selected", managed_path)
	var saved: Dictionary = simulation.snapshot()
	_application.call("_on_world_action", 6)
	if not _check(float(simulation.snapshot()["carbon_dioxide"]) != float(saved["carbon_dioxide"]), "Actual gas action must mutate the loaded checkpoint"):
		return
	_application.call("_open_statistics")
	_application.call("_open_laboratory", "empty")
	_application.call("_on_world_action", 15)
	var recovered: Dictionary = simulation.snapshot()
	if not _check(bool(recovered["ok"]) and bool(recovered["paused"]) and int(recovered["world_time"]) == 4 and _same_gases(saved, recovered), "Explicit Recover must publish the saved paused root"):
		return
	if not _check(not bool(recovered["tracking"]) and (recovered["selected"] as Dictionary).is_empty() and not is_instance_valid(_application.get("_statistics_window")) and (recovered["genotype_preview"] as Dictionary).is_empty(), "Recovery must clear frontend views and selection owners"):
		return
	_application.call("_on_world_action", 6)
	_application.call("_on_world_action", 12)
	var recovered_save: Dictionary = {"operation": str(_application.get("_file_operation")), "path": str(_application.get("_world_path")), "checkpoint_unchanged": FileAccess.get_file_as_bytes(managed_path) == managed_bytes}
	print("BGR_RECOVERED_SAVE_OBSERVATION " + JSON.stringify(recovered_save))
	if not _check(recovered_save["operation"] == "world_save" and str(recovered_save["path"]).is_empty() and bool(recovered_save["checkpoint_unchanged"]), "Save after Recover must request an explicit destination without overwriting its managed checkpoint"):
		return
	var selected_path: String = directory.path_join("selected-recovered-world.bgw")
	_application.call("_on_genotype_file_selected", selected_path)
	var fallback_expected: Dictionary = simulation.snapshot()
	var selected_bytes: PackedByteArray = FileAccess.get_file_as_bytes(selected_path)
	if not _check(FileAccess.file_exists(selected_path) and not selected_bytes.is_empty() and str(_application.get("_world_path")) == selected_path and FileAccess.get_file_as_bytes(managed_path) == managed_bytes, "Selected recovered Save must adopt only its explicit destination"):
		return
	_application.call("_on_world_action", 6)
	_application.call("_on_world_action", 12)
	if not _check(str(_application.get("_file_operation")).is_empty() and str(_application.get("_world_path")) == selected_path and FileAccess.get_file_as_bytes(selected_path) != selected_bytes and FileAccess.get_file_as_bytes(managed_path) == managed_bytes, "Later ordinary Save must update the selected file while retaining the managed checkpoint"):
		return
	var newest_manual_checkpoint: String = str(simulation.automatic_backup_status()["last"]["path"])
	var corrupt: FileAccess = FileAccess.open(newest_manual_checkpoint, FileAccess.WRITE)
	if not _check(corrupt != null, "The owned newest snapshot fixture must open"):
		return
	corrupt.store_string("interrupted or corrupt owned snapshot")
	corrupt.close()
	var rejected_bytes: PackedByteArray = FileAccess.get_file_as_bytes(newest_manual_checkpoint)
	_application.call("_on_world_action", 15)
	var fallback: Dictionary = simulation.snapshot()
	if not _check(int(fallback["world_time"]) == int(fallback_expected["world_time"]) and _same_gases(fallback_expected, fallback) and bool(fallback["paused"]) and not str(simulation.automatic_backup_status()["recovery_warning"]).is_empty() and "Skipped" in (_application.get("_automatic_label") as Label).text and FileAccess.get_file_as_bytes(newest_manual_checkpoint) == rejected_bytes, "Corrupt newest must fall back visibly without changing rejected bytes"):
		return
	saved = fallback
	_application.call("shutdown")
	_application.queue_free()
	await process_frame

	var restarted: BgrSimulation = BgrSimulation.new()
	if not _check(bool(restarted.preferences_initialize()["ok"]) and bool(restarted.configure(827364, 1200, 800, 0)["ok"]), "Fresh process-equivalent session must initialize normally"):
		return
	restarted.configure_automatic_backups(OS.get_user_data_dir(), "")
	if not _check(int(restarted.snapshot()["world_time"]) == 0, "Existing managed files must not silently replace startup World"):
		return
	var restart_recovery: Dictionary = restarted.recover_latest_world()
	if not _check(bool(restart_recovery["ok"]) and int(restart_recovery["world_time"]) == int(saved["world_time"]) and _same_gases(saved, restart_recovery), "Explicit recovery must find committed snapshots after store reopening"):
		return
	restarted.release_world()
	_make_application(restarted)
	if not (await _settings(restarted, 0, true, "2")):
		return
	_application.call("_toggle_pause")
	_application.call("_accept_result", restarted.advance(510))
	_application.call("_toggle_pause")
	var partial: Dictionary = restarted.apply_commands(PackedInt32Array([2, 2, 2, 2]))
	_application.call("_accept_result", partial)
	if not _check(bool(partial["ok"]) and int(partial["commands_completed"]) == 2 and int(partial["tick"]) == 512 and bool(partial["automatic_backup"]["source_save_pending"]), "Multi-step commands must report partial consumption at the unnamed Source boundary"):
		return
	var blocked: Dictionary = restarted.apply_commands(PackedInt32Array([2, 2]))
	if not _check(bool(blocked["ok"]) and int(blocked["commands_completed"]) == 0 and int(blocked["tick"]) == 512, "A pending Save As must not consume subsequent single-step commands"):
		return
	_application.call("_consume_source_backup")
	if not _check(str(_application.get("_file_operation")) == "world_save_as", "Unnamed automatic Source Backup must enter actual Save As"):
		return
	_application.call("_finish_file_operation")
	_application.call("_refresh_automatic_context")
	if not _check(not bool(restarted.automatic_backup_status()["source_save_pending"]) and int(restarted.automatic_backup_status()["attempts"]) == 1 and str(_application.get("_world_path")).is_empty(), "Cancelling Save As must latch the attempt and retain the unnamed world"):
		return
	_application.call("_toggle_pause")
	_application.call("_accept_result", restarted.advance(1024))
	_application.call("_consume_source_backup")
	var path: String = directory.path_join("adjacent.bgw")
	_application.call("_on_genotype_file_selected", path.trim_suffix(".bgw"))
	if not _check(FileAccess.file_exists(path) and str(_application.get("_world_path")) == path and int(restarted.snapshot()["world_time"]) == 4 and not bool(restarted.snapshot()["paused"]), "Accepted automatic Save As must adopt its path and restore prior activity at the barrier"):
		return
	var alternate: String = directory.path_join("alternate.bgw")
	if not _check(bool(restarted.save_world_file(alternate)["ok"]), "An independent named Open fixture must save"):
		return
	var active_bytes: PackedByteArray = FileAccess.get_file_as_bytes(path)
	var next: Dictionary = restarted.advance(512)
	_application.call("_accept_result", next)
	if not _check(bool(next["ok"]) and int(next["world_time"]) == 6 and FileAccess.file_exists(directory.path_join("adjacent@00006.bgw")) and FileAccess.get_file_as_bytes(path) == active_bytes and str(_application.get("_world_path")) == path and not bool(next["paused"]), "Named Source scheduling must write a sibling without replacing or pausing the active world"):
		return
	var failure_target: String = directory.path_join("adjacent@00008.bgw")
	if not _check(DirAccess.make_dir_absolute(failure_target) == OK, "The owned failure directory must be created"):
		return
	var failure: Dictionary = restarted.advance(512)
	_application.call("_accept_result", failure)
	if not _check(bool(failure["ok"]) and int(failure["world_time"]) == 8 and not bool(failure["paused"]) and not str(failure["automatic_backup"]["error"]).is_empty() and not bool(_application.get("_runtime_failed")), "Persistence failure must stay visible and separate from successful biology"):
		return
	_application.call("_accept_result", restarted.advance(256))
	var open_failure: String = directory.path_join("alternate@00004.bgw")
	if not _check(DirAccess.make_dir_absolute(open_failure) == OK, "The newly adopted path failure fixture must be created"):
		return
	_application.call("_on_world_action", 11)
	_application.call("_on_genotype_file_selected", alternate)
	if not _check(str(_application.get("_world_path")) == alternate and int(restarted.snapshot()["world_time"]) == 4 and bool(restarted.snapshot()["paused"]) and not str(restarted.automatic_backup_status()["error"]).is_empty() and not FileAccess.file_exists(directory.path_join("adjacent@00004.bgw")), "Successful Open must adopt its path before paused due failure and retain success"):
		return
	if not (await _settings(restarted, 0, true, "0")):
		return
	if not _check(int(restarted.settings_snapshot()["values"]["BACKUP_DELAY"]) == 2 and int(restarted.automatic_backup_status()["delay"]) == 2 and (_application.get("_settings_report") as RichTextLabel).visible and "BACKUP_DELAY" in (_application.get("_settings_report") as RichTextLabel).text and bool(restarted.snapshot()["ok"]) and not bool(_application.get("_runtime_failed")), "Source settings must reject zero delay, retain its positive value, and report the retained input without invalidating the world"):
		return
	_application.call("shutdown")
	_application.queue_free()
	print("BGR_AUTOMATIC_BACKUP_BOUNDARY_PASS " + str(_checks))
	quit(0)


func _make_application(simulation: BgrSimulation) -> void:
	_application = Main.new()
	_application.set("startup_seed_text", "827364")
	_application.set("startup_dimensions", Vector2i(1200, 800))
	_application.set("startup_attempts", 0)
	_application.call("initialize_session", simulation)
	root.add_child(_application)
	_application.set_process(false)


func _settings(simulation: BgrSimulation, mode: int, enabled: bool, delay: String) -> bool:
	_application.call("_open_settings")
	var window: Window = _application.get("_settings_window") as Window
	(window.get("_backup_mode") as OptionButton).select(mode)
	(window.get("_backup") as CheckBox).set_pressed_no_signal(enabled)
	var fields: Dictionary = window.get("_data")
	var editors: Array[LineEdit] = window.get("_editors")
	for field: Dictionary in fields["fields"]:
		if str(field["key"]) == "BACKUP_DELAY":
			editors[int(field["fixed_order"])].text = delay
	window.call("_submit")
	await process_frame
	return _check(not bool(_application.call("_settings_open")) and int(simulation.settings_snapshot()["automatic_backup_mode"]) == mode, "Actual settings window must accept and close its typed draft")


func _same_model(left: Dictionary, right: Dictionary) -> bool:
	for key: String in ["seed", "tick", "world_time", "paused", "width", "height", "population", "oxygen", "carbon_dioxide", "methane", "segments", "colors", "alphas", "organism_ids", "selected", "tracking", "clipboard_ready"]:
		if left.get(key) != right.get(key):
			return false
	return true


func _same_gases(left: Dictionary, right: Dictionary) -> bool:
	return left["oxygen"] == right["oxygen"] and left["carbon_dioxide"] == right["carbon_dioxide"] and left["methane"] == right["methane"]


func _check(value: bool, message: String) -> bool:
	_checks += 1
	if not value:
		printerr("BGR_AUTOMATIC_BACKUP_BOUNDARY_FAIL " + message)
		if is_instance_valid(_application):
			_application.call("shutdown")
		quit(1)
	return value
