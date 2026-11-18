extends "res://ui/settings_controls_check.gd"

var _preference_signature: Dictionary = {}
var _preference_frame: int = -1
var _preference_samples: int = 0


func _observe() -> Dictionary:
	var observed: Dictionary = super._observe()
	var simulation: BgrSimulation = _launcher.get("_simulation") as BgrSimulation
	observed["preferences"] = simulation.preferences_snapshot()
	observed["preference_path"] = OS.get_user_data_dir().path_join("biogenesis-r.preferences.xml")
	observed["preference_exists"] = FileAccess.file_exists(str(observed["preference_path"]))
	observed["preference_sha256"] = FileAccess.get_sha256(str(observed["preference_path"])) if bool(observed["preference_exists"]) else ""
	observed["crisp_owner"] = bool(simulation.settings_snapshot()["crisp_lines"])
	observed["preference_observation_frame"] = -1
	observed["preference_failure"] = false
	var application: Control = _launcher.get("_main") as Control
	if is_instance_valid(application):
		var canvas: Control = application.get("_canvas") as Control
		var crisp: CheckButton = application.get("_crisp_lines") as CheckButton
		observed["crisp_checkbox"] = crisp.button_pressed
		observed["canvas_crisp"] = bool(canvas.get("_crisp_lines"))
		_add_control(observed, crisp)
		var window: Window = application.get_window()
		observed["actual_geometry"] = [window.position.x, window.position.y, window.size.x, window.size.y]
		var usable: Rect2i = DisplayServer.screen_get_usable_rect(window.current_screen)
		observed["screen_usable"] = [usable.position.x, usable.position.y, usable.size.x, usable.size.y]
		observed["geometry_notice"] = str(_launcher.get("_geometry_notice"))
		observed["timing_text"] = (application.get("_timing_label") as Label).text
		var failure: ConfirmationDialog = application.get("_preference_failure") as ConfirmationDialog
		observed["preference_failure"] = failure.visible
		if failure.visible or not str(observed["decision"]).is_empty() or not str(observed["file_operation"]).is_empty():
			observed["preference_observation_frame"] = Engine.get_process_frames()
		if failure.visible:
			observed["preference_failure_text"] = failure.dialog_text
			for name: String in ["RetryPreferenceSave", "CancelPreferenceClose", "DiscardPreferenceSave"]:
				_add_control(observed, failure.find_child(name, true, false) as Control)
	var signature: Dictionary = {"controls": observed["controls"], "owners": observed["control_native_windows"]}
	var frame: int = Engine.get_process_frames()
	if signature != _preference_signature:
		_preference_signature = signature.duplicate(true)
		_preference_samples = 1
	elif frame != _preference_frame:
		_preference_samples = mini(3, _preference_samples + 1)
	_preference_frame = frame
	observed["settings_geometry_ready"] = _preference_samples >= 3
	for name: String in observed["control_details"]:
		observed["control_ready"][name] = bool(observed["control_details"][name]["inside"]) and _preference_samples >= 3
	return observed


func _checkpoint(label: String) -> void:
	await process_frame
	await RenderingServer.frame_post_draw
	var paths: Array[String] = []
	var windows: Array[Window] = [root]
	var application: Control = _launcher.get("_main") as Control
	if is_instance_valid(application):
		windows.append(application.get_window())
		for property: String in ["_settings_window", "_decision_dialog", "_preference_failure"]:
			var window: Window = application.get(property) as Window
			if is_instance_valid(window) and window.visible:
				windows.append(window)
	for index: int in range(windows.size()):
		var path: String = _output.path_join("%s-%d.png" % [label, index])
		if windows[index].get_texture().get_image().save_png(path) != OK:
			_fail("Cannot save actual preference screenshot")
			return
		paths.append(path)
	var observed: Dictionary = _observe()
	observed["label"] = label
	observed["screenshots"] = paths
	print("BGR_LIFECYCLE_CHECKPOINT " + JSON.stringify(observed, "", true, true))
	_capturing = false
