extends SceneTree

var _launcher: Control
var _lifecycle_status_expected: Dictionary
var _deadline_usec: int = 0
var _output: String = ""
var _last: Dictionary = {}
var _seen: Dictionary = {}
var _seen_laboratories: Array[WeakRef] = []
var _laboratory_accessibility: Dictionary = {}
var _capturing: bool = false
var _last_checkpoint: String = ""
var _native_close_counts: Dictionary = {}
var _geometry_signature: Dictionary = {}
var _geometry_frame: int = -1
var _geometry_samples: int = 0


func _initialize() -> void:
	_output = OS.get_environment("BGR_GUI_OUTPUT")
	if not _output.is_absolute_path() or not DirAccess.dir_exists_absolute(_output):
		_fail("BGR_GUI_OUTPUT must be an existing absolute private directory")
		return
	var status_values: Variant = JSON.parse_string(FileAccess.get_file_as_string("res://ui/main_status_expected.json"))
	if not status_values is Dictionary or status_values.size() != 6:
		_fail("Require independent Main accessibility goldens for six locales")
		return
	_lifecycle_status_expected = status_values
	for language: String in ["en", "es", "ca", "de", "pt", "ru"]:
		var values: Variant = _lifecycle_status_expected.get(language)
		if not values is Dictionary:
			_fail("Missing Main accessibility locale: " + language)
			return
		for key: String in ["R_SETUP_SEED", "R_SETUP_WIDTH", "R_SETUP_HEIGHT", "R_SETUP_ATTEMPTS"]:
			if not values.get(key) is String:
				_fail("Missing Main accessibility golden: " + language + "/" + key)
				return
	var scene: PackedScene = load("res://scenes/launcher.tscn") as PackedScene
	_launcher = scene.instantiate() as Control
	root.add_child(_launcher)
	_deadline_usec = Time.get_ticks_usec() + 200000000


func _process(_delta: float) -> bool:
	if _deadline_usec == 0:
		return false
	if Time.get_ticks_usec() > _deadline_usec:
		_fail("Lifecycle native controls timed out")
		return false
	var observed: Dictionary = _observe()
	if observed != _last:
		_last = observed
		print("BGR_LIFECYCLE_OBS " + JSON.stringify(observed, "", true, true))
	var request_path: String = _output.path_join("checkpoint-request.json")
	if not _capturing and FileAccess.file_exists(request_path):
		var request: Variant = JSON.parse_string(FileAccess.get_file_as_string(request_path))
		if not request is Dictionary or not str(request.get("label", "")).is_valid_identifier():
			_fail("Invalid lifecycle checkpoint request")
			return false
		var label: String = str(request["label"])
		if label != _last_checkpoint:
			_last_checkpoint = label
			_capturing = true
			_checkpoint(label)
	return false


func _observe() -> Dictionary:
	var observed: Dictionary = {"controls": {}, "control_native_windows": {}, "window_position": [0, 0], "session": false}
	observed["display_server"] = DisplayServer.get_name()
	observed["user_data_dir"] = OS.get_user_data_dir()
	for node_name: String in ["StartStopButton", "LauncherQuitButton"]:
		_add_control(observed, _launcher.find_child(node_name, true, false) as Control)
	observed["launcher_window"] = _native_window_info(_launcher.get_window())
	observed["launcher_status"] = (_launcher.get("_status") as Label).text
	observed["start_stop"] = (_launcher.get("_start_stop") as Button).text
	observed["exit_when_closed"] = bool(_launcher.get("_exit_when_closed"))
	var session: BgrSimulation = _launcher.get("_simulation") as BgrSimulation
	observed["session_instance"] = session.get_instance_id()
	var application: Control = _launcher.get("_main") as Control
	var current_id: int = 0
	if is_instance_valid(application):
		current_id = application.get_instance_id()
		if not _seen.has(current_id):
			_seen[current_id] = weakref(application)
		var state: Dictionary = application.get("_state")
		var window: Window = application.get_window()
		observed["session"] = true
		observed["main_session_instance"] = (application.get("_simulation") as BgrSimulation).get_instance_id()
		observed["instance"] = current_id
		observed["native_window"] = not window.is_embedded()
		observed["window_id"] = window.get_window_id()
		observed["simulation_window"] = _native_window_info(window)
		observed["file_operation"] = str(application.get("_file_operation"))
		observed["world_path"] = str(application.get("_world_path"))
		observed["automatic_backup"] = (application.get("_automatic_status") as Dictionary).duplicate(true)
		observed["recover_action_id"] = 15
		var automatic_label: Label = application.get("_automatic_label") as Label
		observed["automatic_text"] = automatic_label.text if automatic_label.visible else ""
		for child: Node in application.get_children():
			if child is Window and child.has_method("_capture") and child.find_child("SettingsAutomaticMode", true, false) != null:
				for control_name: String in ["SettingsAutomaticMode", "SettingsAutomaticBackup", "SettingsOK", "SettingsCancel"]:
					_add_control(observed, child.find_child(control_name, true, false) as Control)
				observed["settings_automatic_mode"] = (child.find_child("SettingsAutomaticMode", true, false) as OptionButton).selected
		observed["save_path"] = str(application.get("_save_path"))
		var file_dialog: FileDialog = application.get("_file_dialog") as FileDialog
		observed["file_dialog"] = {"visible": file_dialog.visible, "native_requested": file_dialog.use_native_dialog, "title": file_dialog.title, "mode": file_dialog.file_mode, "path": file_dialog.current_path}
		if file_dialog.visible:
			_add_control(observed, file_dialog.get_line_edit(), "WorldFileName")
			_add_control(observed, file_dialog.get_ok_button(), "WorldFileOkay")
			_add_control(observed, file_dialog.get_cancel_button(), "WorldFileCancel")
			observed["file_dialog"]["filename_text"] = file_dialog.get_line_edit().text
			observed["file_dialog"]["embedded"] = file_dialog.is_embedded()
		var overwrite: ConfirmationDialog = application.get("_overwrite_dialog") as ConfirmationDialog
		observed["overwrite"] = {"visible": overwrite.visible, "title": overwrite.title, "text": overwrite.dialog_text}
		var world_popup: PopupMenu = (application.get("_world_menu") as MenuButton).get_popup()
		var world_focused: int = world_popup.get_focused_item()
		observed["world_menu"] = {"visible": world_popup.visible, "focused_id": world_popup.get_item_id(world_focused) if world_focused >= 0 else -1}
		observed["statistics_visible"] = is_instance_valid(application.get("_statistics_window"))

		observed["shutting_down"] = bool(application.get("_shutting_down"))
		observed["decision_visible"] = (application.get("_decision_dialog") as Window).visible
		observed["decision"] = str(application.get("_decision_operation"))
		observed["awaiting_save"] = bool(application.get("_awaiting_world_save"))
		observed["tick"] = int(state.get("tick", -1))
		observed["world_time"] = int(state.get("world_time", -1))
		for gas: String in ["oxygen", "carbon_dioxide", "methane"]:
			observed[gas] = float(state.get(gas, 0.0))
		observed["paused"] = bool(state.get("paused", true))
		observed["population"] = int(state.get("population", -1))
		observed["selected"] = not state.get("selected", {}).is_empty()
		observed["tracking"] = bool(state.get("tracking", false))
		observed["clipboard_ready"] = bool(state.get("clipboard_ready", false))
		var model: Dictionary = {}
		for key: String in ["tick", "seed", "paused", "width", "height", "initial_attempts", "population", "oxygen", "carbon_dioxide", "methane", "segments", "colors", "alphas", "organism_ids"]:
			model[key] = state.get(key)
		observed["model_sha256"] = JSON.stringify(model, "", true, true).sha256_text()
		observed["seed_text"] = (application.get("_seed_input") as LineEdit).text
		observed["world_seed_text"] = str(state.get("seed", ""))
		observed["applied_width"] = int(state.get("width", 0))
		observed["applied_height"] = int(state.get("height", 0))
		observed["applied_attempts"] = int(state.get("initial_attempts", -1))
		observed["texts"] = {}
		observed["accessibility"] = {}
		var names: Dictionary = {"SeedInput": "R_SETUP_SEED", "WidthInput": "R_SETUP_WIDTH", "HeightInput": "R_SETUP_HEIGHT", "AttemptsInput": "R_SETUP_ATTEMPTS"}
		var values: Dictionary = _lifecycle_status_expected.get(str(application.get("_main_language")), _lifecycle_status_expected["en"])
		for field: String in ["SeedInput", "WidthInput", "HeightInput", "AttemptsInput"]:
			var input: LineEdit = application.find_child(field, true, false) as LineEdit
			observed["texts"][field] = input.text
			observed["accessibility"][field] = input.accessibility_name
			if input.accessibility_name != str(values[str(names[field])]):
				_fail("Main accessibility name differs from its visible label: " + field)
		var error_label: Label = application.get("_error_label") as Label
		observed["error"] = error_label.text if error_label.visible else ""
		_laboratory_accessibility.clear()
		observed["laboratories"] = _laboratory_count(application)
		observed["laboratory_accessibility"] = _laboratory_accessibility.duplicate()
		for node_name: String in ["WorldCanvas", "CreateWorldButton", "PauseButton", "StepButton", "ResetButton", "QuitButton", "SeedInput", "WidthInput", "HeightInput", "AttemptsInput", "GeneticsMenu", "WorldActionsMenu", "TrackOrganismButton", "ConfirmWorldDecision", "CancelWorldDecision", "SaveWorldDecision"]:
			_add_control(observed, application.find_child(node_name, true, false) as Control)
		_add_control(observed, overwrite.get_ok_button())
		_add_control(observed, overwrite.get_cancel_button())
		var genetics: MenuButton = application.find_child("GeneticsMenu", true, false) as MenuButton
		observed["genetics_menu"] = genetics.get_popup().visible
		var focused: int = genetics.get_popup().get_focused_item()
		observed["genetics_focused_id"] = genetics.get_popup().get_item_id(focused) if focused >= 0 else -1
		var canvas: Control = application.get("_canvas") as Control
		var screen_transform: Transform2D = _physical_transform(canvas)
		var offset: Vector2 = canvas.get("_offset")
		var zoom: float = float(canvas.get("_zoom"))
		var points: PackedFloat64Array = state.get("segments", PackedFloat64Array())
		var identities: PackedInt64Array = state.get("organism_ids", PackedInt64Array())
		observed["pick_points"] = []
		var recorded: Dictionary = {}
		for index: int in range(identities.size()):
			if recorded.has(identities[index]):
				continue
			recorded[identities[index]] = true
			var screen: Vector2 = Vector2(points[index * 4], points[index * 4 + 1]) * zoom + offset
			if Rect2(Vector2.ZERO, canvas.size).has_point(screen):
				var absolute: Vector2 = screen_transform * screen
				observed["pick_points"].append([absolute.x, absolute.y])
			if observed["pick_points"].size() >= 32:
				break
	else:
		var stopped: Dictionary = session.snapshot()
		observed["world_ready"] = bool(stopped["world_ready"])
		observed["session_ready"] = bool(stopped["session_ready"])
		observed["stopped_seed_text"] = str(stopped.get("seed", ""))
		observed["stopped_owners_clear"] = not bool(stopped["clipboard_ready"]) and not bool(stopped["tracking"]) and stopped["selected"].is_empty() and stopped["genotype_preview"].is_empty()
	var retired: int = 0
	for instance: int in _seen:
		var reference: WeakRef = _seen[instance]
		if instance != current_id and reference.get_ref() == null:
			retired += 1
	observed["sessions_seen"] = _seen.size()
	observed["sessions_freed"] = retired
	var freed_laboratories: int = 0
	for reference: WeakRef in _seen_laboratories:
		if reference.get_ref() == null:
			freed_laboratories += 1
	observed["laboratories_freed"] = freed_laboratories
	var frame: int = Engine.get_process_frames()
	var signature: Dictionary = {"controls": observed["controls"], "instance": current_id}
	if signature != _geometry_signature:
		_geometry_signature = signature.duplicate(true)
		_geometry_samples = 1
	elif frame != _geometry_frame:
		_geometry_samples = mini(3, _geometry_samples + 1)
	_geometry_frame = frame
	observed["geometry_ready"] = _geometry_samples >= 3
	return observed


func _native_window_info(window: Window) -> Dictionary:
	var identity: int = window.get_instance_id()
	if not _native_close_counts.has(identity):
		_native_close_counts[identity] = 0
		window.close_requested.connect(_record_native_close.bind(identity))
	return {"instance_id": identity, "window_id": window.get_window_id(), "native_handle": DisplayServer.window_get_native_handle(DisplayServer.WINDOW_HANDLE, window.get_window_id()), "title": window.title, "close_requested_count": int(_native_close_counts[identity])}


func _record_native_close(identity: int) -> void:
	_native_close_counts[identity] = int(_native_close_counts.get(identity, 0)) + 1


func _native_window(control: Control) -> Window:
	var window: Window = control.get_window()
	while window.is_embedded():
		var parent: Node = window.get_parent()
		if parent == null:
			_fail("Embedded window has no native ancestor")
			return null
		window = parent.get_window()
	return window


func _physical_transform(control: Control) -> Transform2D:
	var window: Window = _native_window(control)
	if window == null:
		return Transform2D()
	var transform: Transform2D = control.get_viewport().get_screen_transform() * control.get_global_transform_with_canvas()
	transform.origin += Vector2(DisplayServer.window_get_position(window.get_window_id()))
	return transform


func _add_control(observed: Dictionary, control: Control, alias: String = "") -> void:
	if not is_instance_valid(control) or not control.is_visible_in_tree() or not control.get_window().visible:
		return
	var rect: Rect2 = _physical_transform(control) * Rect2(Vector2.ZERO, control.size)
	observed["controls"][str(control.name) if alias.is_empty() else alias] = [rect.position.x, rect.position.y, rect.size.x, rect.size.y]
	var window: Window = _native_window(control)
	if window == null:
		return
	var origin: Vector2i = DisplayServer.window_get_position(window.get_window_id())
	observed["control_native_windows"][str(control.name) if alias.is_empty() else alias] = {"title": window.title, "window_id": window.get_window_id(), "client_origin": [origin.x, origin.y]}


func _laboratory_count(node: Node) -> int:
	var count: int = 1 if node.get_script() == preload("res://ui/laboratory_window.gd") else 0
	if count == 1:
		var known: bool = false
		for reference: WeakRef in _seen_laboratories:
			if reference.get_ref() == node:
				known = true
				break
		if not known:
			_seen_laboratories.append(weakref(node))
		var names: Dictionary = {"Genome_symmetry": "Symmetry: ", "GeneLength": "Length (2..18)", "GeneColor": "Color"}
		for field: String in names:
			var control: Control = node.find_child(field, true, false) as Control
			if is_instance_valid(control):
				_laboratory_accessibility[field] = control.accessibility_name
				if control.accessibility_name != str(names[field]):
					_fail("Laboratory accessibility name differs: " + field)
	for child: Node in node.get_children():
		count += _laboratory_count(child)
	return count


func _checkpoint(label: String) -> void:
	await process_frame
	await RenderingServer.frame_post_draw
	var paths: Array[String] = []
	var windows: Array[Window] = [root]
	var simulation_window: Window = _launcher.get("_simulation_window") as Window
	if is_instance_valid(simulation_window):
		windows.append(simulation_window)
		var application: Control = _launcher.get("_main") as Control
		var decision: Window = application.get("_decision_dialog") as Window
		if decision.visible:
			windows.append(decision)
		for property: String in ["_file_dialog", "_overwrite_dialog"]:
			var dialog: Window = application.get(property) as Window
			if dialog.visible:
				windows.append(dialog)
	for index: int in range(windows.size()):
		var path: String = _output.path_join("%s-%d.png" % [label, index])
		if windows[index].get_texture().get_image().save_png(path) != OK:
			_fail("Cannot save lifecycle screenshot")
			return
		paths.append(path)
	var observed: Dictionary = _observe()
	observed["label"] = label
	observed["screenshots"] = paths
	print("BGR_LIFECYCLE_CHECKPOINT " + JSON.stringify(observed, "", true, true))
	_capturing = false


func _fail(message: String) -> void:
	printerr("BGR_GUI_FAIL " + message)
	quit(1)
