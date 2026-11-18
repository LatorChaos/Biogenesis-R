extends SceneTree

var _checks: int = 0


func _check(value: bool, message: String) -> void:
	_checks += 1
	if not value:
		printerr("BGR_STARTUP_LAUNCHER_FAIL " + message)
		quit(1)
		assert(value, message)


func _initialize() -> void:
	await process_frame
	var launcher: Control = (load("res://scenes/launcher.tscn") as PackedScene).instantiate()
	root.add_child(launcher)
	for _frame: int in range(4):
		await process_frame
	if OS.get_environment("BGR_STARTUP_CONFLICT") == "1":
		_check(bool(launcher.get("_launcher_only")) and not bool(launcher.get("_resume_managed")), "Conflicting opt-in flags stay in Launcher")
		_check(not is_instance_valid(launcher.get("_main")), "Conflicting flags create no simulation")
	else:
		var application: Control = launcher.get("_main") as Control
		_check(is_instance_valid(application), "Actual Launcher scene opens Main")
		application.set_process(false)
		var simulation: BgrSimulation = application.get("_simulation") as BgrSimulation
		_check(bool(application.get("startup_resume_managed")) and bool(application.get("_startup_recovery_succeeded")), "Actual command-line flag reaches successful recovery")
		_check(not bool(launcher.get("_resume_managed")), "Launcher consumes recovery mode once per launch")
		_check(int(simulation.snapshot()["world_time"]) == 4 and not bool(simulation.snapshot()["paused"]), "Launcher restores newest saved time and resumes")
		_check(int(simulation.snapshot()["initial_attempts"]) == 3125 and int(simulation.settings_snapshot()["genesis"][2]) == 3125, "Fresh frontend genesis remains 3125 after restore")
		_check(float(application.get("_pending_ticks")) >= 0.0 and str(application.get("_world_path")).is_empty(), "Managed file is not the ordinary Save destination")
		application.call("_on_world_action", 15)
		_check(bool(simulation.snapshot()["paused"]) and int(simulation.snapshot()["world_time"]) == 4, "Existing explicit Recover remains paused after startup")
		await launcher.call("_stop_simulation")
	launcher.queue_free()
	await process_frame
	print("BGR_STARTUP_LAUNCHER " + JSON.stringify({"result": "PASS", "checks": _checks, "failures": []}))
	quit(0)
