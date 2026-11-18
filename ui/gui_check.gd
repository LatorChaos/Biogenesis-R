extends SceneTree

const MODEL_FIELDS: Array[String] = ["seed", "tick", "paused", "width", "height", "population", "oxygen", "carbon_dioxide", "segments", "colors", "alphas", "organism_ids"]

var _application: Control
var _initial: Dictionary = {}
var _stage: int = -1
var _deadline_usec: int = 0
var _stable_since_usec: int = 0
var _paused_tick: int = -1
var _window_size: Vector2i
var _output_directory: String = ""
var _failed: bool = false
var _emitting: bool = false
var _fit_zoom: float = 0.0
var _pan_origin: Vector2
var _pan_zoom: float = 0.0
var _last_world_decision: Dictionary = {}


func _initialize() -> void:
	_output_directory = OS.get_environment("BGR_GUI_OUTPUT")
	if not _output_directory.is_empty() and (not _output_directory.is_absolute_path() or not DirAccess.dir_exists_absolute(_output_directory)):
		_fail("BGR_GUI_OUTPUT must name an existing absolute directory")
		return
	var scene: PackedScene = load("res://scenes/main.tscn") as PackedScene
	_application = scene.instantiate() as Control
	_application.set("startup_dimensions", Vector2i(1200, 800))
	_application.set("startup_attempts", 96)
	root.add_child(_application)
	_deadline_usec = Time.get_ticks_usec() + 120000000


func _process(_delta: float) -> bool:
	if _failed:
		return false
	if Time.get_ticks_usec() > _deadline_usec:
		_fail("Native controls timed out at stage " + str(_stage))
		return false
	if _emitting:
		return false
	var state: Dictionary = _application.get("_state")
	if state.is_empty():
		return false
	var tick: int = int(state["tick"])
	var paused: bool = bool(state["paused"])
	if (_application.get("_decision_dialog") as Window).visible:
		var observed: Dictionary = _observation(state)
		if observed != _last_world_decision:
			_last_world_decision = observed
			print("BGR_GUI_DECISION " + JSON.stringify(observed))
		return false
	_last_world_decision = {}
	match _stage:
		-1:
			if not paused or tick != 0:
				_fail("Initial state is not paused at tick zero")
				return false
			_initial = state.duplicate(true)
			_window_size = root.size
			_stage = 0
			_emitting = true
			_capture_and_print("BGR_GUI_READY", "ready", _observation(state))
		0:
			if paused and tick == 1:
				_event("STEP", state)
		1:
			if not paused and tick > 1:
				_event("RUNNING", state)
		2:
			if paused:
				if _stable_since_usec == 0:
					_stable_since_usec = Time.get_ticks_usec()
					_paused_tick = tick
				elif tick != _paused_tick:
					_fail("Paused model advanced")
				elif Time.get_ticks_usec() - _stable_since_usec >= 200000:
					_event("PAUSED", state)
		3:
			if paused and tick == 0:
				if not _same_initial(state):
					_fail("Reset changed the seeded initial model")
					return false
				_event("RESET", state)
		4:
			var canvas: Control = _application.find_child("WorldCanvas", true, false) as Control
			if not bool(canvas.get("_fit_enabled")):
				var zoom: float = float(canvas.get("_zoom"))
				if not is_finite(zoom) or zoom <= _fit_zoom:
					_fail("Wheel zoom did not increase the actual scale")
				else:
					_event("ZOOM", state)
		5:
			var canvas: Control = _application.find_child("WorldCanvas", true, false) as Control
			if bool(canvas.get("_fit_enabled")):
				_event("FIT", state)
		6:
			if root.size != _window_size:
				var canvas: Control = _application.find_child("WorldCanvas", true, false) as Control
				if canvas.size.x <= 0.0 or canvas.size.y <= 0.0:
					_fail("Resize removed the world viewport")
				else:
					_event("RESIZED", state)
		7:
			var canvas: Control = _application.find_child("WorldCanvas", true, false) as Control
			var offset: Vector2 = canvas.get("_offset")
			if offset.distance_to(_pan_origin) > 5.0:
				if not is_equal_approx(float(canvas.get("_zoom")), _pan_zoom) or not _same_initial(state):
					_fail("Pan changed zoom or the authoritative model")
				else:
					_event("PAN", state)
		8:
			if root.size == Vector2i(800, 660):
				_event("MINIMUM", state)
		9:
			var width: LineEdit = _application.find_child("WidthInput", true, false) as LineEdit
			var error: Label = _application.get("_error_label")
			if width.text == "0" and error.visible:
				if error.text.is_empty() or not _same_initial(state):
					_fail("Invalid configuration changed the current model or omitted its error")
				else:
					_event("CONFIG_ERROR", state)
		10:
			var width: LineEdit = _application.find_child("WidthInput", true, false) as LineEdit
			var error: Label = _application.get("_error_label")
			if width.text == "1200" and not error.visible and paused and tick == 0:
				if _same_initial(state) or int(state["seed"]) != int(_initial["seed"]) or int(state["width"]) != int(_initial["width"]) or int(state["height"]) != int(_initial["height"]) or int(state["initial_attempts"]) != int(_initial["initial_attempts"]) or int(state["population"]) <= 0:
					_fail("Create did not continue the stream with the requested configuration")
				else:
					_event("NEW_WORLD", state)
		11:
			if paused and tick == 1:
				_event("BUTTON_STEP", state)
		12:
			if not paused and tick > 1:
				_event("BUTTON_RUNNING", state)
		13:
			if paused:
				if _stable_since_usec == 0:
					_stable_since_usec = Time.get_ticks_usec()
					_paused_tick = tick
				elif tick != _paused_tick:
					_fail("Button-paused model advanced")
				elif Time.get_ticks_usec() - _stable_since_usec >= 200000:
					_event("BUTTON_PAUSED", state)
		14:
			if paused and tick == 0:
				if not _same_initial(state):
					_fail("Button reset changed the seeded model")
				else:
					_event("BUTTON_RESET", state)
	return false


func _event(stage_name: String, state: Dictionary) -> void:
	_stage += 1
	_emitting = true
	if stage_name == "BUTTON_RUNNING":
		_stable_since_usec = 0
		_paused_tick = -1
	var observation: Dictionary = _observation(state)
	observation["stage"] = stage_name
	_capture_and_print("BGR_GUI_EVENT", stage_name.to_lower(), observation)


func _capture_and_print(prefix: String, stage_name: String, observation: Dictionary) -> void:
	await process_frame
	await RenderingServer.frame_post_draw
	var current: Dictionary = _application.get("_state")
	observation.merge(_observation(current), true)
	var canvas: Control = _application.find_child("WorldCanvas", true, false) as Control
	if stage_name == "reset":
		_fit_zoom = float(canvas.get("_zoom"))
	elif stage_name == "resized":
		_pan_origin = canvas.get("_offset")
		_pan_zoom = float(canvas.get("_zoom"))
	if stage_name in ["fit", "resized", "new_world"] and not _fitted_inside(current, canvas):
		_fail("Fit did not place the complete world inside the canvas")
		return
	if stage_name in ["minimum", "config_error", "new_world"] and not _controls_inside_viewport():
		return
	if not _output_directory.is_empty():
		var image: Image = root.get_texture().get_image()
		if stage_name in ["ready", "reset", "new_world"] and int(current["population"]) > 0:
			if not _visible_world_content(image, current, canvas):
				_fail("Populated paused world has no visible organisms")
				return
		var path: String = _output_directory.path_join(stage_name + ".png")
		var error: Error = image.save_png(path)
		if error != OK:
			_fail("Viewport screenshot failed: " + error_string(error))
			return
		observation["screenshot"] = path
		observation["screenshot_size"] = [image.get_width(), image.get_height()]
	print(prefix + " " + JSON.stringify(observation))
	_emitting = false


func _visible_world_content(image: Image, state: Dictionary, canvas: Control) -> bool:
	var zoom: float = float(canvas.get("_zoom"))
	var origin: Vector2 = canvas.global_position + (canvas.get("_offset") as Vector2)
	var end: Vector2 = origin + Vector2(float(state["width"]), float(state["height"])) * zoom
	var left: int = maxi(0, ceili(origin.x) + 4)
	var top: int = maxi(0, ceili(origin.y) + 4)
	var right: int = mini(image.get_width(), floori(end.x) - 4)
	var bottom: int = mini(image.get_height(), floori(end.y) - 4)
	var background: int = Color.BLACK.to_rgba32()
	for y: int in range(top, bottom):
		for x: int in range(left, right):
			if image.get_pixel(x, y).to_rgba32() != background:
				return true
	return false


func _same_initial(state: Dictionary) -> bool:
	for key: String in MODEL_FIELDS:
		if state[key] != _initial[key]:
			return false
	return true


func _fitted_inside(state: Dictionary, canvas: Control) -> bool:
	var zoom: float = float(canvas.get("_zoom"))
	var offset: Vector2 = canvas.get("_offset")
	var expected: float = minf(canvas.size.x / float(state["width"]), canvas.size.y / float(state["height"])) * 0.92
	return is_finite(zoom) and zoom > 0.0 and absf(zoom - expected) <= expected * 0.000001 and offset.x >= -0.01 and offset.y >= -0.01 and offset.x + float(state["width"]) * zoom <= canvas.size.x + 0.01 and offset.y + float(state["height"]) * zoom <= canvas.size.y + 0.01


func _controls_inside_viewport() -> bool:
	var bounds: Rect2 = Rect2(Vector2.ZERO, Vector2(root.size))
	for node: Node in _application.find_children("*", "Control", true, false):
		var control: Control = node as Control
		if control.is_visible_in_tree():
			var rect: Rect2 = control.get_global_rect()
			if rect.position.x < -0.1 or rect.position.y < -0.1 or rect.end.x > bounds.end.x + 0.1 or rect.end.y > bounds.end.y + 0.1:
				_fail("Control exceeds the minimum viewport: " + str(control.name))
				return false
	return true


func _observation(state: Dictionary) -> Dictionary:
	var controls: Dictionary = {}
	for node_name: String in ["PauseButton", "StepButton", "ResetButton", "FitButton", "ZoomInButton", "QuitButton", "WorldCanvas", "SeedInput", "WidthInput", "HeightInput", "AttemptsInput", "CreateWorldButton"]:
		var control: Control = _application.find_child(node_name, true, false) as Control
		var rect: Rect2 = control.get_global_rect()
		controls[node_name] = [rect.position.x, rect.position.y, rect.size.x, rect.size.y]
	var canvas: Control = _application.find_child("WorldCanvas", true, false) as Control
	var offset: Vector2 = canvas.get("_offset")
	var observed: Dictionary = {"tick": int(state["tick"]), "paused": bool(state["paused"]), "population": int(state["population"]), "zoom": float(canvas.get("_zoom")), "offset": [offset.x, offset.y], "controls": controls, "window_position": [root.position.x, root.position.y], "window_size": [root.size.x, root.size.y]}
	observed["configuration"] = [int(state["seed"]), int(state["width"]), int(state["height"]), int(state["initial_attempts"])]
	var model: Dictionary = {}
	for key: String in MODEL_FIELDS:
		model[key] = state[key]
	observed["model_sha256"] = JSON.stringify(model, "", true, true).sha256_text()
	var decision: ConfirmationDialog = _application.get("_decision_dialog") as ConfirmationDialog
	observed["decision_visible"] = decision.visible
	observed["decision_controls_ready"] = false
	observed["decision"] = str(_application.get("_decision_operation"))
	if decision.visible:
		var decision_rects: Array[Rect2] = []
		for node_name: String in ["ConfirmWorldDecision", "CancelWorldDecision", "SaveWorldDecision"]:
			var control: Control = _application.find_child(node_name, true, false) as Control
			var window: Window = control.get_window()
			while window.is_embedded():
				var parent: Node = window.get_parent()
				if parent == null:
					_fail("Embedded decision has no native window ancestor")
					return observed
				window = parent.get_window()
			var transform: Transform2D = control.get_viewport().get_screen_transform() * control.get_global_transform_with_canvas()
			transform.origin += Vector2(DisplayServer.window_get_position(window.get_window_id())) - Vector2(root.position)
			var rect: Rect2 = transform * Rect2(Vector2.ZERO, control.size)
			controls[node_name] = [rect.position.x, rect.position.y, rect.size.x, rect.size.y]
			decision_rects.append(rect)
		var ready: bool = true
		for index: int in range(decision_rects.size()):
			var candidate_rect: Rect2 = decision_rects[index]
			if not is_finite(candidate_rect.position.x) or not is_finite(candidate_rect.position.y) or not is_finite(candidate_rect.size.x) or not is_finite(candidate_rect.size.y) or candidate_rect.size.x <= 0.0 or candidate_rect.size.y <= 0.0:
				ready = false
			for other_index: int in range(index):
				var other_rect: Rect2 = decision_rects[other_index]
				if candidate_rect.position.x < other_rect.end.x and candidate_rect.end.x > other_rect.position.x and candidate_rect.position.y < other_rect.end.y and candidate_rect.end.y > other_rect.position.y:
					ready = false
		observed["decision_controls_ready"] = ready
	return observed


func _fail(message: String) -> void:
	_failed = true
	push_error(message)
	print("BGR_GUI_FAIL " + JSON.stringify({"message": message, "stage": _stage}))
	quit(1)
