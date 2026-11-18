extends SceneTree

const Main = preload("res://ui/main.gd")
const StartupArguments = preload("res://ui/startup_arguments.gd")
const DisplayPreferences = preload("res://ui/display_preferences.gd")

var _application: Control
var _checks: int = 0
var _output: String


func _check(value: bool, message: String) -> void:
	_checks += 1
	if not value:
		printerr("BGR_STARTUP_MANAGED_FAIL " + message)
		quit(1)
		assert(value, message)


func _make_application(resume_managed: bool, attempts: int = 512) -> BgrSimulation:
	var simulation: BgrSimulation = BgrSimulation.new()
	_check(bool(simulation.preferences_initialize()["ok"]), "Private preferences initialize")
	_application = Main.new()
	_application.set("startup_seed_text", "827364")
	_application.set("startup_dimensions", Vector2i(1200, 800))
	_application.set("startup_attempts", attempts)
	_application.set("startup_resume_managed", resume_managed)
	_check(bool(_application.call("initialize_session", simulation)), "Main accepts the caller session")
	root.add_child(_application)
	_application.set_process(false)
	return simulation


func _close_application() -> void:
	_application.call("shutdown")
	_application.queue_free()
	await process_frame


func _same_world(expected: Dictionary, actual: Dictionary) -> void:
	for key: String in ["world_time", "population", "width", "height", "oxygen", "carbon_dioxide", "methane", "segments", "colors", "alphas", "organism_ids"]:
		_check(expected[key] == actual[key], "Saved World retains " + key)


func _encoded(simulation: BgrSimulation, name: String) -> PackedByteArray:
	var path: String = _output.path_join(name)
	_check(bool(simulation.save_world_file(path)["ok"]), "World comparison saves " + name)
	return FileAccess.get_file_as_bytes(path)


func _initialize() -> void:
	await process_frame
	_output = OS.get_environment("BGR_STARTUP_CHECK_OUTPUT")
	_check(_output.is_absolute_path() and DirAccess.dir_exists_absolute(_output), "Require private comparison output")
	for arguments: PackedStringArray in [PackedStringArray(), PackedStringArray(["827364"]), PackedStringArray(["--launcher"]), PackedStringArray(["--launcher", "827364"])]:
		var parsed: Dictionary = StartupArguments.parse(arguments)
		_check(bool(parsed["valid"]) and not bool(parsed["resume_managed"]), "Ordinary argument behavior remains")
	for arguments: PackedStringArray in [PackedStringArray(["--resume-managed"]), PackedStringArray(["--resume-managed", "827364"])]:
		var parsed: Dictionary = StartupArguments.parse(arguments)
		_check(bool(parsed["valid"]) and bool(parsed["resume_managed"]) and not bool(parsed["launcher_only"]), "Opt-in mode parses")
	for arguments: PackedStringArray in [PackedStringArray(["--resume-managed", "--resume-managed"]), PackedStringArray(["--launcher", "--resume-managed"]), PackedStringArray(["--resume-managed", "--launcher"]), PackedStringArray(["--resume-managed=1"]), PackedStringArray(["827364", "--resume-managed"]), PackedStringArray(["--resume-managed", "bad"]), PackedStringArray(["--resume-managed", "1", "2"])]:
		var parsed: Dictionary = StartupArguments.parse(arguments)
		_check(not bool(parsed["valid"]) and not bool(parsed["resume_managed"]) and bool(parsed["launcher_only"]), "Invalid opt-in cannot launch a replacement")
	var defaults: Control = Main.new()
	_check(int(defaults.get("startup_attempts")) == 3125 and defaults.get("startup_dimensions") == Vector2i(3500, 3500) and not bool(defaults.get("startup_resume_managed")), "Default startup values remain unchanged")
	defaults.free()
	var simulation: BgrSimulation = _make_application(false, 0)
	await process_frame
	_check(bool(simulation.snapshot()["paused"]) and int(simulation.snapshot()["world_time"]) == 0, "Ordinary headless startup remains paused and new")
	_check(not DirAccess.dir_exists_absolute(OS.get_user_data_dir().path_join("managed-recovery")), "Ordinary startup does not create recovery storage")
	await _close_application()

	simulation = _make_application(true)
	await process_frame
	_check(bool(simulation.snapshot()["paused"]) and not bool(_application.get("_startup_recovery_succeeded")), "Empty managed store cannot autoplay")
	_check((_application.get("_error_label") as Label).visible and not (_application.get("_pause_button") as Button).disabled, "Failure leaves visible error and usable controls")
	_check(int(simulation.snapshot()["initial_attempts"]) == 512 and int(simulation.settings_snapshot()["genesis"][2]) == 512, "Failed restore retains configured genesis attempts")
	_application.call("_toggle_pause")
	_check(not bool(simulation.snapshot()["paused"]), "Manual Resume remains usable after failed startup")
	_application.call("_toggle_pause")
	await _close_application()

	simulation = _make_application(false)
	var draft: BgrSettingsDraft = simulation.settings_open()
	var controls: Dictionary = draft.snapshot()
	var texts: PackedStringArray = controls["texts"]
	texts[(controls["keys"] as PackedStringArray).find("BACKUP_DELAY")] = "2"
	_check(bool(draft.replace_controls({"texts": texts, "locale": controls["locale"], "automatic_backup": true, "automatic_backup_mode": 1, "hardware": controls["hardware"], "disable_framebuffer": controls["disable_framebuffer"]})["ok"]), "Managed fixture controls validate")
	_check(bool(simulation.settings_apply(draft, Callable(_application, "_settings_locale_changed"))["ok"]), "Managed fixture settings apply")
	_application.call("_toggle_pause")
	var advanced: Dictionary = simulation.advance(512)
	_application.call("_accept_result", advanced)
	var older: String = str(simulation.automatic_backup_status()["last"]["path"])
	var older_state: Dictionary = simulation.snapshot()
	advanced = simulation.advance(512)
	_application.call("_accept_result", advanced)
	var newest: String = str(simulation.automatic_backup_status()["last"]["path"])
	var newest_state: Dictionary = simulation.snapshot()
	_check(bool(advanced["ok"]) and int(older_state["world_time"]) == 2 and int(newest_state["world_time"]) == 4 and older != newest, "Two real checkpoints exist")
	var newest_bytes: PackedByteArray = FileAccess.get_file_as_bytes(newest)
	var older_bytes: PackedByteArray = FileAccess.get_file_as_bytes(older)
	await _close_application()

	var preferences: RefCounted = DisplayPreferences.new()
	preferences.values["cinema_enabled"] = false
	_check(bool(preferences.save_settings()["ok"]), "Cinema-off startup preference saves")
	var control: BgrSimulation = BgrSimulation.new()
	_check(bool(control.preferences_initialize()["ok"]) and bool(control.start_session(827364, 1200, 800, 512)["ok"]), "Independent native caller initializes")
	_check(bool(control.open_world_file(newest)["ok"]), "Independent native caller opens checkpoint")
	var expected_bytes: PackedByteArray = _encoded(control, "direct-native-open.bgw")
	simulation = _make_application(true)
	_same_world(newest_state, simulation.snapshot())
	_check(bool(_application.get("_startup_recovery_succeeded")) and bool(simulation.snapshot()["paused"]), "Recovery completes before deferred playback")
	_check(_encoded(simulation, "startup-restored.bgw") == expected_bytes, "Startup retains the complete saved genetic graph")
	_check(int(simulation.snapshot()["initial_attempts"]) == 512 and int(simulation.settings_snapshot()["genesis"][2]) == 512, "Successful restore preserves 512 and future genesis defaults")
	_check(float(_application.get("_pending_ticks")) == 0.0 and str(_application.get("_world_path")).is_empty(), "Recovery resets debt and keeps managed Save destination unadopted")
	await process_frame
	_check(not bool(simulation.snapshot()["paused"]) and not bool(_application.get("_cinema_enabled")), "Recovered World resumes with Cinema disabled")
	_check(bool(simulation.advance(1)["ok"]) and int(simulation.snapshot()["tick"]) > 0, "Restored World can actually advance")
	_application.call("_toggle_pause")
	_check(bool(simulation.snapshot()["paused"]), "Manual pause works after startup Resume")
	_check(FileAccess.get_file_as_bytes(newest) == newest_bytes and FileAccess.get_file_as_bytes(older) == older_bytes, "Startup preserves both checkpoint byte streams")
	_check(bool(_application.call("_apply_reset")) and bool(control.apply_commands(PackedInt32Array([3]))["ok"]), "Reset retains both caller configurations")
	_check(int(simulation.snapshot()["initial_attempts"]) == 512, "Reset retains configured initial attempts")
	# Advance was outside the identity comparison, so compare New using a fresh paired caller below
	await _close_application()
	control.release_world()

	simulation = _make_application(true)
	control = BgrSimulation.new()
	_check(bool(control.preferences_initialize()["ok"]) and bool(control.start_session(827364, 1200, 800, 512)["ok"]) and bool(control.open_world_file(newest)["ok"]), "Paired continuation caller initializes")
	_check(bool(_application.call("_apply_reset")) and bool(control.apply_commands(PackedInt32Array([3]))["ok"]), "Paired Reset executes")
	_check(_encoded(simulation, "startup-reset.bgw") == _encoded(control, "direct-reset.bgw"), "Recovery adds no RNG draws before Reset")
	_check(bool(simulation.new_game(1200, 800, 512)["ok"]) and bool(control.new_game(1200, 800, 512)["ok"]), "Paired New executes")
	_check(_encoded(simulation, "startup-new.bgw") == _encoded(control, "direct-new.bgw"), "Recovery preserves New configuration and RNG")
	await _close_application()
	control.release_world()

	var corrupt: FileAccess = FileAccess.open(newest, FileAccess.WRITE)
	_check(corrupt != null, "Newest fixture opens for deliberate corruption")
	corrupt.store_string("invalid newest test checkpoint")
	corrupt.close()
	var rejected: PackedByteArray = FileAccess.get_file_as_bytes(newest)
	simulation = _make_application(true)
	_same_world(older_state, simulation.snapshot())
	_check(bool(_application.get("_startup_recovery_succeeded")) and not str(simulation.automatic_backup_status()["recovery_warning"]).is_empty(), "Invalid newest falls back visibly")
	await process_frame
	_check(not bool(simulation.snapshot()["paused"]) and FileAccess.get_file_as_bytes(newest) == rejected, "Valid fallback resumes without rewriting rejected data")
	await _close_application()
	corrupt = FileAccess.open(older, FileAccess.WRITE)
	_check(corrupt != null, "Older fixture opens for deliberate corruption")
	corrupt.store_string("invalid older test checkpoint")
	corrupt.close()
	var rejected_older: PackedByteArray = FileAccess.get_file_as_bytes(older)
	simulation = _make_application(true)
	await process_frame
	_check(bool(simulation.snapshot()["paused"]) and not bool(_application.get("_startup_recovery_succeeded")) and (_application.get("_error_label") as Label).visible, "All-invalid managed data never autoplays")
	_check(FileAccess.get_file_as_bytes(newest) == rejected and FileAccess.get_file_as_bytes(older) == rejected_older, "Failed startup preserves all rejected data")
	await _close_application()
	# Restore owned test fixture bytes only for the separate ordinary-entry smoke
	for path: String in [older, newest]:
		var fixture: FileAccess = FileAccess.open(path, FileAccess.WRITE)
		_check(fixture != null, "Owned smoke fixture restores")
		fixture.store_buffer(older_bytes if path == older else newest_bytes)
		fixture.close()
	print("BGR_STARTUP_MANAGED " + JSON.stringify({"result": "PASS", "checks": _checks, "failures": []}))
	quit(0)
