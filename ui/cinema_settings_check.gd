extends SceneTree

const SettingsWindow = preload("res://ui/settings_window.gd")
const DisplayPreferences = preload("res://ui/display_preferences.gd")
const MainLabels = preload("res://ui/main_labels.gd")
var _checks: int = 0
var _failures: Array[String] = []
var _applied: Dictionary = {}
var _locale_events: int = 0
var _simulation: BgrSimulation
var _preferences: DisplayPreferences


func _check(condition: bool, message: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(message)


func _open() -> SettingsWindow:
	var window: SettingsWindow = SettingsWindow.new()
	window.simulation = _simulation
	window.draft = _simulation.settings_open()
	window.display_preferences = _preferences
	window.locale_callback = func(_payload: Dictionary) -> void: _locale_events += 1
	window.applied.connect(func(result: Dictionary) -> void: _applied = result)
	root.add_child(window)
	return window


func _initialize() -> void:
	_run.call_deferred()


func _run() -> void:
	_simulation = BgrSimulation.new()
	_check(bool(_simulation.preferences_initialize()["ok"]), "Initialize private preferences")
	_check(bool(_simulation.configure(827364, 1000, 900, 0)["ok"]), "Create an empty controlled world")
	_preferences = DisplayPreferences.new()
	_check(bool(_preferences.load_settings()["ok"]), "Load display defaults")
	_check(_preferences.values["camera_motion_percent"] == 12.5 and _preferences.values["camera_settled_hold_seconds"] == 200.0, "Defaults do not implement the requested ratios")
	var before: Dictionary = _simulation.snapshot()
	var settings: Dictionary = _simulation.settings_snapshot()
	var window: SettingsWindow = _open()
	await process_frame
	_check(window.get("_tabs").get_tab_count() == 8 and window.get("_editors").size() == 132, "Cinema replaces original Parameters fields")
	(window.find_child("CinemaMotionPercent", true, false) as SpinBox).value = 25.0
	(window.find_child("CinemaSettledHoldSeconds", true, false) as SpinBox).value = 450.0
	(window.find_child("SettingsCancel", true, false) as Button).pressed.emit()
	await process_frame
	_check(_preferences.values["camera_motion_percent"] == 12.5 and not FileAccess.file_exists(DisplayPreferences.PATH), "Cancel persists the camera draft")
	_check(_simulation.settings_snapshot() == settings and _simulation.snapshot() == before, "Cancel changes biology or simulation state")
	window = _open()
	await process_frame
	var motion: SpinBox = window.find_child("CinemaMotionPercent", true, false) as SpinBox
	var hold: SpinBox = window.find_child("CinemaSettledHoldSeconds", true, false) as SpinBox
	_check(motion.value == 12.5 and hold.value == 200.0, "Reopen retains canceled values")
	motion.get_line_edit().text = "25"
	hold.get_line_edit().text = "450"
	(window.find_child("SettingsOK", true, false) as Button).pressed.emit()
	var apply_error: String = str(window.get("_error").text)
	await process_frame
	_check(bool(_applied.get("ok", false)) and not _applied.has("display_error"), "Parameters OK fails the separate display save: " + str(_applied) + "/" + apply_error)
	var restored: DisplayPreferences = DisplayPreferences.new()
	_check(bool(restored.load_settings()["ok"]) and restored.values["camera_motion_percent"] == 25.0 and restored.values["camera_settled_hold_seconds"] == 450.0, "Typed camera controls do not persist")
	_check(_simulation.snapshot() == before, "Camera controls change world or RNG-dependent output")
	window = _open()
	await process_frame
	_check((window.find_child("CinemaMotionPercent", true, false) as SpinBox).value == 25.0 and (window.find_child("CinemaSettledHoldSeconds", true, false) as SpinBox).value == 450.0, "Reopen loses committed values")
	for language: String in ["en", "es", "ca", "de", "pt", "ru"]:
		window.apply_locale(language)
		_check(window.get("_tabs").get_tab_title(7) == MainLabels.message("R_CINEMA_TAB", language) and not window.get("_cinema_note").text.begins_with("!"), "Cinema labels are missing in a supported locale")
	(window.find_child("SettingsDefaults", true, false) as Button).pressed.emit()
	_check((window.find_child("CinemaMotionPercent", true, false) as SpinBox).value == 12.5 and (window.find_child("CinemaSettledHoldSeconds", true, false) as SpinBox).value == 200.0, "Defaults do not restore camera defaults")
	(window.find_child("SettingsCancel", true, false) as Button).pressed.emit()
	await process_frame
	_check(_preferences.values["camera_motion_percent"] == 25.0, "Canceled defaults change committed camera values")
	var bytes: PackedByteArray = FileAccess.get_file_as_bytes(DisplayPreferences.PATH)
	for field: String in ["camera_motion_percent", "camera_settled_hold_seconds"]:
		var prior: Variant = _preferences.values[field]
		for invalid: Variant in [NAN, INF, -1.0, true, "20", 5000.0]:
			_preferences.values[field] = invalid
			_check(not bool(_preferences.save_settings()["ok"]) and FileAccess.get_file_as_bytes(DisplayPreferences.PATH) == bytes, "Invalid camera value overwrites preferences")
		_preferences.values[field] = prior
	print("BGR_CINEMA_SETTINGS " + JSON.stringify({"checks": _checks, "failures": _failures, "result": "PASS" if _failures.is_empty() else "FAIL"}))
	quit(0 if _failures.is_empty() else 1)
