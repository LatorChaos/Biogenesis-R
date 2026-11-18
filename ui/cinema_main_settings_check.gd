extends SceneTree
const Main = preload("res://ui/main.gd")
const DisplayPreferences = preload("res://ui/display_preferences.gd")

class TimedMain extends Main:
	func _clock_usec() -> int:
		return 1000000

class FailingPreferences extends DisplayPreferences:
	func save_settings() -> Dictionary:
		return {"ok": false, "error": "Controlled display write failure"}

var _checks: int = 0
var _failures: Array[String] = []

func _check(condition: bool, message: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(message)

func _initialize() -> void:
	_run.call_deferred()

func _run() -> void:
	var simulation: BgrSimulation = BgrSimulation.new()
	_check(bool(simulation.preferences_initialize()["ok"]), "Initialize the private session")
	var application: TimedMain = TimedMain.new()
	application.startup_seed_text = "827364"
	application.startup_dimensions = Vector2i(96, 96)
	application.startup_attempts = 0
	_check(application.initialize_session(simulation), "Initialize the actual Main session")
	root.add_child(application)
	application.set_process(false)
	await process_frame
	await process_frame
	var director: RefCounted = application.get("_director")
	_check(director.motion_rate_scale == 0.125 and director.settled_hold_seconds == 200.0, "Main does not apply default camera settings")
	var before: Dictionary = simulation.snapshot()
	(application.find_child("SettingsButton", true, false) as Button).pressed.emit()
	await process_frame
	var window: Window = application.get("_settings_window")
	(window.find_child("CinemaMotionPercent", true, false) as SpinBox).get_line_edit().text = "25"
	(window.find_child("CinemaSettledHoldSeconds", true, false) as SpinBox).get_line_edit().text = "450"
	(window.find_child("SettingsOK", true, false) as Button).pressed.emit()
	await process_frame
	_check(director.motion_rate_scale == 0.25 and director.settled_hold_seconds == 450.0, "Main ignores applied Parameters camera controls")
	_check(simulation.snapshot() == before, "Camera Parameters changes the simulation")
	application.call("_manual_camera_override")
	_check(application.get("_manual_camera_until") == 41.0 and director.hold_seconds == 20.0, "Camera tuning changes the existing manual override or feeding cooldown")
	var restored: DisplayPreferences = DisplayPreferences.new()
	_check(bool(restored.load_settings()["ok"]) and restored.values["camera_motion_percent"] == 25.0 and restored.values["camera_settled_hold_seconds"] == 450.0, "Main camera choices do not persist")
	var failed: FailingPreferences = FailingPreferences.new()
	failed.values = restored.values.duplicate()
	application.set("_display_preferences", failed)
	(application.find_child("SettingsButton", true, false) as Button).pressed.emit()
	await process_frame
	window = application.get("_settings_window")
	(window.find_child("CinemaMotionPercent", true, false) as SpinBox).get_line_edit().text = "50"
	(window.find_child("SettingsOK", true, false) as Button).pressed.emit()
	await process_frame
	_check(director.motion_rate_scale == 0.25 and failed.values["camera_motion_percent"] == 25.0, "Failed display persistence changes the active camera policy")
	_check((application.get("_error_label") as Label).visible and "Controlled display write failure" in (application.get("_error_label") as Label).text, "Main hides the separate display write failure")
	_check(simulation.snapshot() == before, "Display write failure changes world state")
	application.queue_free()
	await process_frame
	print("BGR_CINEMA_MAIN_SETTINGS " + JSON.stringify({"checks": _checks, "failures": _failures, "result": "PASS" if _failures.is_empty() else "FAIL"}))
	quit(0 if _failures.is_empty() else 1)
