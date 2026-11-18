extends "res://ui/gui_check.gd"

var _last_observation: Dictionary = {}
var _capturing: bool = false
var _last_checkpoint: String = ""


func _process(_delta: float) -> bool:
	if _failed:
		return false
	if Time.get_ticks_usec() > _deadline_usec:
		_fail("Crisp native controls timed out")
		return false
	var state: Dictionary = _application.get("_state")
	if state.is_empty():
		return false
	if _initial.is_empty():
		_initial = state.duplicate(true)
	var observed: Dictionary = _observation(state)
	if observed != _last_observation:
		_last_observation = observed
		print("BGR_CRISP_OBS " + JSON.stringify(observed, "", true, true))
	if not _capturing and FileAccess.file_exists(_output_directory.path_join("checkpoint-request.json")):
		var request: Variant = JSON.parse_string(FileAccess.get_file_as_string(_output_directory.path_join("checkpoint-request.json")))
		if not request is Dictionary or not request.has("label"):
			_fail("Invalid crisp checkpoint request")
			return false
		var label: String = str(request["label"])
		if not label.is_valid_identifier():
			_fail("Invalid crisp checkpoint label")
			return false
		if label != _last_checkpoint:
			_last_checkpoint = label
			_capturing = true
			_checkpoint(label)
	return false


func _observation(state: Dictionary) -> Dictionary:
	var result: Dictionary = super._observation(state)
	var checkbox: CheckButton = _application.find_child("CrispLines", true, false) as CheckButton
	var canvas: Control = _application.find_child("WorldCanvas", true, false) as Control
	for node_name: String in ["CrispLines", "ZoomOutButton"]:
		var control: Control = _application.find_child(node_name, true, false) as Control
		var rect: Rect2 = control.get_global_rect()
		result["controls"][node_name] = [rect.position.x, rect.position.y, rect.size.x, rect.size.y]
	result["checkbox"] = checkbox.button_pressed
	result["canvas_crisp"] = bool(canvas.get("_crisp_lines"))
	result["same_initial"] = _same_initial(state)
	result["fit"] = bool(canvas.get("_fit_enabled"))
	var focus: Control = root.gui_get_focus_owner()
	result["focus"] = "" if focus == null else str(focus.name)
	var texts: Dictionary = {}
	for node_name: String in ["SeedInput", "WidthInput", "HeightInput", "AttemptsInput"]:
		texts[node_name] = (_application.find_child(node_name, true, false) as LineEdit).text
	result["texts"] = texts
	return result


func _checkpoint(label: String) -> void:
	await process_frame
	await RenderingServer.frame_post_draw
	var state: Dictionary = _application.get("_state")
	var canvas: Control = _application.find_child("WorldCanvas", true, false) as Control
	if not _controls_inside_viewport():
		return
	if bool(canvas.get("_fit_enabled")) and not _fitted_inside(state, canvas):
		_fail("Crisp controls lost fitted world extents")
		return
	var image: Image = root.get_texture().get_image()
	if bool(state["paused"]) and int(state["population"]) > 0 and bool(canvas.get("_fit_enabled")):
		if not _visible_world_content(image, state, canvas):
			_fail("Crisp controls show no populated world content")
			return
	var path: String = _output_directory.path_join(label + ".png")
	if image.save_png(path) != OK:
		_fail("Cannot preserve crisp native screenshot")
		return
	var observed: Dictionary = _observation(state)
	observed["label"] = label
	observed["screenshot"] = path
	print("BGR_CRISP_CHECKPOINT " + JSON.stringify(observed, "", true, true))
	_capturing = false
