extends "res://ui/lifecycle_check.gd"

var _settings_windows: Array[WeakRef] = []
var _settings_drafts: Array[WeakRef] = []
var _settings_signature: Dictionary = {}
var _settings_frame: int = -1
var _settings_samples: int = 0


func _initialize() -> void:
	super._initialize()
	_deadline_usec = Time.get_ticks_usec() + 380000000


func _observe() -> Dictionary:
	var observed: Dictionary = super._observe()
	var session: BgrSimulation = _launcher.get("_simulation") as BgrSimulation
	observed["settings_state"] = session.settings_snapshot()
	observed["settings_open"] = false
	if not observed.has("control_details"):
		observed["control_details"] = {}
	observed["control_ready"] = {}
	observed["settings_controls"] = {}
	observed["settings_editors"] = {}
	observed["settings_labels"] = {}
	observed["settings_tabs"] = []
	var application: Control = _launcher.get("_main") as Control
	if is_instance_valid(application):
		var native_size: Vector2i = application.get_window().size
		observed["simulation_size"] = [native_size.x, native_size.y]
		_add_control(observed, application.find_child("SettingsButton", true, false) as Control)
		var report: RichTextLabel = application.get("_settings_report") as RichTextLabel
		observed["retained_report"] = report.text if report.visible else ""
		observed["requested_delay_ms"] = float(maxi(1, int(application.get("_stored_delay_ms"))))
		observed["pending_seconds"] = float(application.get("_pending_ticks")) * float(application.get("_pacing").current_delay_ms) / 1000.0
		var state: Dictionary = application.get("_state")
		observed["gases"] = [state.get("oxygen"), state.get("carbon_dioxide"), state.get("methane")]
		var dialog: Window = application.get("_settings_window") as Window
		if is_instance_valid(dialog) and dialog.visible:
			observed["settings_open"] = true
			observed["settings_window"] = _native_window_info(dialog)
			observed["settings_window"]["size"] = [dialog.size.x, dialog.size.y]
			observed["settings_window"]["position"] = [dialog.position.x, dialog.position.y]
			observed["settings_window"]["exclusive"] = dialog.exclusive
			observed["settings_window"]["transient"] = dialog.transient
			_track_weak(_settings_windows, dialog)
			var token: BgrSettingsDraft = dialog.get("draft") as BgrSettingsDraft
			_track_weak(_settings_drafts, token)
			var focus: Control = dialog.gui_get_focus_owner()
			observed["settings_focus"] = str(focus.name) if is_instance_valid(focus) else ""
			var tabs: TabContainer = dialog.get("_tabs") as TabContainer
			observed["settings_tab"] = tabs.current_tab
			var bar: TabBar = tabs.get_tab_bar()
			for index: int in range(tabs.get_tab_count()):
				observed["settings_tabs"].append(tabs.get_tab_title(index))
				var rect: Rect2 = _physical_transform(bar) * bar.get_tab_rect(index)
				_add_rect(observed, "SettingsTab_" + str(index), rect, bar)
			var data: Dictionary = dialog.get("_data")
			var editors: Array = dialog.get("_editors")
			var labels: Array = dialog.get("_labels")
			for field: Dictionary in data["fields"]:
				var index: int = int(field["fixed_order"])
				var key: String = str(field["key"])
				var editor: LineEdit = editors[index] as LineEdit
				var label: Label = labels[index] as Label
				observed["settings_editors"][key] = {"text": editor.text, "editable": editor.editable, "focus_mode": editor.focus_mode, "index": index, "tab": str(field["tab"])}
				observed["settings_labels"][key] = label.text
				_add_control(observed, editor)
			for name: String in ["SettingsLanguage", "SettingsAutomaticBackup", "SettingsLegacyGraphics", "SettingsDisableFramebuffer", "SettingsOK", "SettingsCancel", "SettingsDefaults"]:
				var control: Control = dialog.find_child(name, true, false) as Control
				_add_control(observed, control)
				if control is OptionButton:
					var popup: PopupMenu = control.get_popup()
					var focused: int = popup.get_focused_item()
					observed["settings_controls"][name] = {"selected": control.selected, "popup": popup.visible, "focused_index": focused, "focused_id": popup.get_item_id(focused) if focused >= 0 else -1, "item_count": popup.get_item_count()}
				elif control is BaseButton:
					observed["settings_controls"][name] = {"pressed": control.button_pressed, "disabled": control.disabled}
			observed["settings_error"] = (dialog.get("_error") as Label).text
	var signature: Dictionary = {"controls": observed["controls"], "owners": observed["control_native_windows"], "settings_open": observed["settings_open"]}
	var frame: int = Engine.get_process_frames()
	if signature != _settings_signature:
		_settings_signature = signature.duplicate(true)
		_settings_samples = 1
	elif frame != _settings_frame:
		_settings_samples = mini(3, _settings_samples + 1)
	_settings_frame = frame
	observed["settings_geometry_ready"] = _settings_samples >= 3
	for name: String in observed["control_details"]:
		observed["control_ready"][name] = bool(observed["control_details"][name]["inside"]) and _settings_samples >= 3
	observed["settings_seen"] = _settings_windows.size()
	observed["settings_freed"] = _freed(_settings_windows)
	observed["settings_drafts_freed"] = _freed(_settings_drafts)
	return observed


func _track_weak(references: Array[WeakRef], value: Object) -> void:
	for reference: WeakRef in references:
		if reference.get_ref() == value:
			return
	references.append(weakref(value))


func _freed(references: Array[WeakRef]) -> int:
	var count: int = 0
	for reference: WeakRef in references:
		if reference.get_ref() == null:
			count += 1
	return count


func _add_control(observed: Dictionary, control: Control, alias: String = "") -> void:
	if not is_instance_valid(control) or not control.is_visible_in_tree() or not control.get_window().visible:
		return
	var rect: Rect2 = _physical_transform(control) * Rect2(Vector2.ZERO, control.size)
	_add_rect(observed, str(control.name) if alias.is_empty() else alias, rect, control)


func _add_rect(observed: Dictionary, name: String, rect: Rect2, control: Control) -> void:
	var window: Window = _native_window(control)
	if window == null:
		return
	var origin: Vector2i = DisplayServer.window_get_position(window.get_window_id())
	var clip: Rect2 = Rect2(Vector2(origin), Vector2(window.size))
	var scroll: String = ""
	var parent: Node = control.get_parent()
	while parent != null and parent != window:
		if parent is Control and parent.clip_contents:
			var bounds: Rect2 = _physical_transform(parent) * Rect2(Vector2.ZERO, parent.size)
			clip = clip.intersection(bounds)
		if parent is ScrollContainer and scroll.is_empty():
			scroll = str(parent.name)
		parent = parent.get_parent()
	observed["controls"][name] = [rect.position.x, rect.position.y, rect.size.x, rect.size.y]
	observed["control_native_windows"][name] = {"title": window.title, "window_id": window.get_window_id(), "native_handle": DisplayServer.window_get_native_handle(DisplayServer.WINDOW_HANDLE, window.get_window_id()), "client_origin": [origin.x, origin.y]}
	if not observed.has("control_details"):
		observed["control_details"] = {}
	observed["control_details"][name] = {"inside": clip.encloses(rect), "clip": [clip.position.x, clip.position.y, clip.size.x, clip.size.y], "scroll": scroll}


func _checkpoint(label: String) -> void:
	await process_frame
	await RenderingServer.frame_post_draw
	var paths: Array[String] = []
	var windows: Array[Window] = [root]
	var application: Control = _launcher.get("_main") as Control
	if is_instance_valid(application):
		windows.append(application.get_window())
		var settings: Window = application.get("_settings_window") as Window
		if is_instance_valid(settings) and settings.visible:
			windows.append(settings)
		var decision: Window = application.get("_decision_dialog") as Window
		if decision.visible:
			windows.append(decision)
	for index: int in range(windows.size()):
		var path: String = _output.path_join("%s-%d.png" % [label, index])
		if windows[index].get_texture().get_image().save_png(path) != OK:
			_fail("Cannot save actual settings screenshot")
			return
		paths.append(path)
	var observed: Dictionary = _observe()
	observed["label"] = label
	observed["screenshots"] = paths
	print("BGR_LIFECYCLE_CHECKPOINT " + JSON.stringify(observed, "", true, true))
	_capturing = false
