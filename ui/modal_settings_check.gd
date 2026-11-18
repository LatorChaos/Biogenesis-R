extends "res://ui/native_frame_loop.gd"

const Main = preload("res://ui/main.gd")
const DisplayPreferences = preload("res://ui/display_preferences.gd")

var _keyboard: Node
var _checks: int = 0
var _open_count: int = 0
var _failures: Array[String] = []
var _observations: Array[Dictionary] = []
var _window: Window
var _application: Main
var _notified: Dictionary = {}
var _simulation: BgrSimulation


func _initialize() -> void:
	_run.call_deferred()


func _check(value: bool, message: String) -> void:
	_checks += 1
	if not value:
		_failures.append(message)


func _frames() -> void:
	await process_frame
	await process_frame
	await process_frame


func _click(control: Control, fullscreen_request: bool = false) -> void:
	var window: Window = control.get_window()
	while window.is_embedded():
		window = window.get_parent().get_window()
	if not _notified.has(window.get_instance_id()):
		window.notify_mouse_entered()
		_notified[window.get_instance_id()] = true
	var position: Vector2 = control.get_viewport().get_screen_transform() * control.get_global_transform_with_canvas() * (control.size * 0.5)
	var motion: InputEventMouseMotion = InputEventMouseMotion.new()
	motion.window_id = window.get_window_id()
	motion.position = position
	motion.global_position = position
	motion.relative = Vector2.ONE
	Input.parse_input_event(motion)
	Input.flush_buffered_events()
	await process_frame
	if fullscreen_request:
		_window.mode = Window.MODE_FULLSCREEN
		_check(_window.mode == Window.MODE_FULLSCREEN, "Real Window samples fullscreen at Parameters admission")
		_observations.append({"fullscreen_admission_mode": _window.mode})
	for down: bool in [true, false]:
		var event: InputEventMouseButton = InputEventMouseButton.new()
		event.window_id = window.get_window_id()
		event.position = position
		event.global_position = position
		event.button_index = MOUSE_BUTTON_LEFT
		event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
		event.pressed = down
		Input.parse_input_event(event)
		if not fullscreen_request:
			await process_frame
	await _frames()


func _key(window: Window, code: Key, unicode: int = 0, control: bool = false) -> void:
	var native_window: Window = window
	while native_window.is_embedded():
		native_window = native_window.get_parent().get_window()
	var native_id: int = native_window.get_window_id()
	for down: bool in [true, false]:
		if not is_instance_valid(native_window):
			break
		var event: InputEventKey = InputEventKey.new()
		event.window_id = native_id
		event.keycode = code
		event.physical_keycode = code
		event.unicode = unicode if down else 0
		event.pressed = down
		event.ctrl_pressed = control
		Input.parse_input_event(event)
		await process_frame
	await _frames()


func _open(fullscreen_request: bool = false) -> Window:
	(_application.get("_ambient_shell") as Control).call("show_tools")
	await _frames()
	if _open_count == 0:
		await _click(_application.find_child("SettingsButton", true, false) as Button)
	else:
		if fullscreen_request:
			_window.mode = Window.MODE_FULLSCREEN
			_check(_window.mode == Window.MODE_FULLSCREEN, "Real Window samples fullscreen at Parameters admission")
			_observations.append({"fullscreen_admission_mode": _window.mode})
		_application.call("_open_settings")
		await _frames()
	_open_count += 1
	var dialog: Window = _application.get("_settings_window") as Window
	_check(is_instance_valid(dialog), "Actual Main command creates the real SettingsWindow")
	return dialog


func _shape(dialog: Window, embedded: bool) -> void:
	_check(dialog.visible and dialog.transient and dialog.exclusive, "Settings remains visible, transient and exclusive")
	_check(dialog.is_embedded() == embedded, "Settings uses the requested embedded/native boundary")
	_check(dialog.get("_editors").size() == 132 and dialog.get("_tabs").get_tab_count() == 8, "All original editors and reviewed Cinema tab remain")
	_check(dialog.size.x >= 640 and dialog.size.y >= 460 and dialog.size.x <= _window.size.x and dialog.size.y <= _window.size.y, "Parameters geometry fits its real parent window")
	if embedded:
		_check(dialog.gui_embed_subwindows and not dialog.force_native, "Embedded Parameters owns embedded child popups")
		_check(Rect2i(Vector2i.ZERO, _window.size).encloses(Rect2i(dialog.position, dialog.size)), "Embedded Parameters is inside the fullscreen viewport")
	_observations.append({"parent_mode": _window.mode, "parent_embeds": _window.gui_embed_subwindows,
		"embedded": dialog.is_embedded(), "dialog_id": dialog.get_window_id(),
		"dialog_position": [dialog.position.x, dialog.position.y], "dialog_size": [dialog.size.x, dialog.size.y]})


func _finish() -> void:
	if is_instance_valid(_window):
		_window.queue_free()
	await _frames()
	print("BGR_MODAL_KEYBOARD " + JSON.stringify({"status": "PASS" if _failures.is_empty() else "FAIL",
		"checks": _checks, "failures": _failures, "observations": _observations,
		"native_WM_stacking_claim": false, "physical_Pi": false}))
	quit(0 if _failures.is_empty() else 1)


func _run() -> void:
	_keyboard = root.get_node("TouchKeyboard")
	_keyboard.call("set_enabled", false)
	var preferences: DisplayPreferences = DisplayPreferences.new()
	preferences.values["fullscreen"] = false
	_check(bool(preferences.save_settings()["ok"]), "Prepare a private windowed display profile")
	_simulation = BgrSimulation.new()
	_check(bool(_simulation.preferences_initialize()["ok"]), "Initialize an independent Rust preference profile")
	_application = Main.new()
	_application.startup_seed_text = "827364"
	_application.startup_dimensions = Vector2i(7000, 7000)
	_application.startup_attempts = 0
	_check(_application.initialize_session(_simulation), "Initialize the actual empty Main session")
	_window = Window.new()
	_window.hide()
	_window.force_native = true
	_window.size = Vector2i(1280, 800)
	root.add_child(_window)
	_window.add_child(_application)
	_application.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_application.set_process(false)
	_window.popup_centered()
	await _frames()
	var before: Dictionary = _simulation.snapshot()
	var initial_delay: int = int(_simulation.settings_snapshot()["values"]["DELAY"])
	var before_file: String = OS.get_user_data_dir().path_join("modal-before.bgw")
	_check(bool(_simulation.save_world_file(before_file)["ok"]), "Save the complete initial World and RNG")
	_check(_window.mode == Window.MODE_WINDOWED and not _window.gui_embed_subwindows, "Initial actual window is native and windowed")
	var dialog: Window = await _open()
	if not is_instance_valid(dialog):
		await _finish()
		return
	_shape(dialog, false)
	await _key(dialog, KEY_ESCAPE)
	_check(not is_instance_valid(_application.get("_settings_window")) and not _window.gui_embed_subwindows, "Native Cancel removes Settings and preserves embedding")
	await _click(_application.find_child("StatisticsButton", true, false) as Button)
	var statistics: Window = _application.get("_statistics_window") as Window
	_check(is_instance_valid(statistics) and statistics.visible, "Keep the real nonmodal Statistics window open")
	var statistics_rect: Rect2i = Rect2i(statistics.position, statistics.size)
	dialog = await _open(true)
	if not is_instance_valid(dialog):
		await _finish()
		return
	_shape(dialog, true)
	_check(not statistics.visible and Rect2i(statistics.position, statistics.size) == statistics_rect, "Fullscreen Parameters hides native Statistics without changing its geometry")
	await RenderingServer.frame_post_draw
	_check(_window.get_texture().get_image().save_png(OS.get_environment("BGR_MODAL_OUTPUT").path_join("embedded-settings.png")) == OK, "Capture actual Parameters inside the parent viewport")
	_check(_window.gui_embed_subwindows, "Fullscreen Parameters temporarily enables parent embedding")
	_application.call("_open_settings")
	_check(_application.get("_settings_window") == dialog, "Repeated open preserves one active draft and Window")
	var delay: LineEdit = dialog.find_child("Setting_DELAY", true, false) as LineEdit
	await _click(delay)
	await _key(dialog, KEY_A, 0, true)
	await _key(dialog, KEY_NONE, "3".unicode_at(0))
	_check(delay.text == "3", "Embedded real editor receives public Window key input")
	await _key(dialog, KEY_ESCAPE)
	_check(not is_instance_valid(_application.get("_settings_window")) and not _window.gui_embed_subwindows, "Embedded Escape restores original parent embedding")
	_check(int(_simulation.settings_snapshot()["values"]["DELAY"]) == initial_delay, "Embedded Cancel discards the biological settings draft")
	_check(statistics.visible and Rect2i(statistics.position, statistics.size) == statistics_rect, "Embedded Cancel restores native Statistics after restoring parent embedding")
	dialog = await _open(true)
	if not is_instance_valid(dialog):
		await _finish()
		return
	delay = dialog.find_child("Setting_DELAY", true, false) as LineEdit
	await _click(delay)
	await _key(dialog, KEY_A, 0, true)
	await _key(dialog, KEY_NONE, "4".unicode_at(0))
	var ok_button: Button = dialog.find_child("SettingsOK", true, false) as Button
	var ok_observation: Dictionary = {"ok_presses": 0, "ok_gui_events": 0, "delay_text": delay.text,
		"ok_rect": str(ok_button.get_global_rect()), "ok_transform": str(ok_button.get_viewport().get_screen_transform()),
		"dialog_position": str(dialog.position), "dialog_id": dialog.get_window_id(), "owner_id": _window.get_window_id(),
		"owner_focus": _window.has_focus(), "dialog_focus": dialog.has_focus(), "statistics_focus": statistics.has_focus(),
		"shell_hold": (_application.get("_ambient_shell") as Control).get("hold_open"), "gui_trace": []}
	ok_button.pressed.connect(func() -> void: ok_observation["ok_presses"] += 1)
	ok_button.gui_input.connect(func(event: InputEvent) -> void:
		ok_observation["ok_gui_events"] += 1
		var item: Dictionary = {"class": event.get_class()}
		if event is InputEventMouseButton:
			item.merge({"pressed": event.pressed, "button": event.button_index, "position": str(event.position), "global": str(event.global_position), "canceled": event.canceled})
		if event is InputEventMouseMotion:
			item["position"] = str(event.position)
		ok_observation["gui_trace"].append(item))
	await _click(ok_button)
	if is_instance_valid(dialog):
		ok_observation["error"] = str(dialog.get("_error").text)
		ok_observation["focus"] = str(dialog.gui_get_focus_owner())
		await RenderingServer.frame_post_draw
		_window.get_texture().get_image().save_png(OS.get_environment("BGR_MODAL_OUTPUT").path_join("ok-after.png"))
	_observations.append(ok_observation)
	_check(not is_instance_valid(_application.get("_settings_window")) and not _window.gui_embed_subwindows, "Embedded OK removes Settings and restores parent embedding")
	_check(int(_simulation.settings_snapshot()["values"]["DELAY"]) == 4, "Embedded OK applies through the original settings transaction")
	_check(statistics.visible and Rect2i(statistics.position, statistics.size) == statistics_rect, "Embedded OK restores native Statistics after restoring parent embedding")
	if is_instance_valid(_application.get("_settings_window")):
		await _finish()
		return
	await _key(statistics, KEY_ESCAPE)
	_check(not is_instance_valid(_application.get("_statistics_window")), "Ordinary Statistics Escape releases the retained statistics viewer")
	_window.gui_embed_subwindows = true
	dialog = await _open(true)
	if not is_instance_valid(dialog):
		await _finish()
		return
	_shape(dialog, true)
	await _key(dialog, KEY_ESCAPE)
	_check(_window.gui_embed_subwindows, "Closing Settings preserves an already embedded parent configuration")
	_window.mode = Window.MODE_WINDOWED
	await _frames()
	dialog = await _open()
	if not is_instance_valid(dialog):
		await _finish()
		return
	_shape(dialog, false)
	await _key(dialog, KEY_ESCAPE)
	_check(_window.gui_embed_subwindows, "Native windowed Settings preserves existing parent embedding")
	_window.gui_embed_subwindows = false
	print("BGR_MODAL_PROGRESS keyboard-start")
	_keyboard.call("set_enabled", true)
	dialog = await _open(true)
	if not is_instance_valid(dialog):
		await _finish()
		return
	_shape(dialog, true)
	var locale: OptionButton = dialog.get("_locale") as OptionButton
	await _click(locale)
	var locale_popup: PopupMenu = locale.get_popup()
	print("BGR_MODAL_PROGRESS popup-open")
	_check(locale_popup.visible and locale_popup.is_embedded(), "Real locale popup remains embedded inside Parameters")
	await _key(locale_popup, KEY_ESCAPE)
	print("BGR_MODAL_PROGRESS popup-close")
	_check(is_instance_valid(dialog) and dialog.visible and not locale_popup.visible, "Popup Escape preserves the active Parameters draft")
	if not is_instance_valid(dialog):
		await _finish()
		return
	delay = dialog.find_child("Setting_DELAY", true, false) as LineEdit
	await _click(delay)
	var panel: Control = dialog.find_child("TouchKeyboard", true, false) as Control
	print("BGR_MODAL_PROGRESS field-focus")
	_check(is_instance_valid(panel) and panel.visible and panel.get_window() == dialog and delay.has_focus(), "Corrected keyboard shares the embedded editor Window without focus theft")
	await _click(panel.find_child("TouchSelectAll", true, false) as Button)
	await _click(panel.find_child("TouchMode", true, false) as Button)
	await _click(panel.find_child("TouchCharacter51", true, false) as Button)
	print("BGR_MODAL_PROGRESS typed-three")
	_check(delay.text == "3", "Public keyboard clicks edit the actual embedded Parameters field")
	await RenderingServer.frame_post_draw
	_check(_window.get_texture().get_image().save_png(OS.get_environment("BGR_MODAL_OUTPUT").path_join("embedded-keyboard.png")) == OK, "Capture real embedded Parameters and keyboard composition")
	await _click(panel.find_child("TouchCancel", true, false) as Button)
	print("BGR_MODAL_PROGRESS cancelled")
	_check(not is_instance_valid(_application.get("_settings_window")) and not _window.gui_embed_subwindows, "Keyboard Escape cancels embedded Parameters and restores embedding")
	_check(int(_simulation.settings_snapshot()["values"]["DELAY"]) == 4, "Keyboard Cancel discards the edited Parameters draft")
	dialog = await _open(true)
	if not is_instance_valid(dialog):
		await _finish()
		return
	delay = dialog.find_child("Setting_DELAY", true, false) as LineEdit
	await _click(delay)
	panel = dialog.find_child("TouchKeyboard", true, false) as Control
	await _click(panel.find_child("TouchSelectAll", true, false) as Button)
	await _click(panel.find_child("TouchMode", true, false) as Button)
	await _click(panel.find_child("TouchCharacter54", true, false) as Button)
	print("BGR_MODAL_PROGRESS typed-six")
	_check(delay.text == "6", "Reopened embedded editor receives a fresh keyboard session")
	await _click(panel.find_child("TouchHide", true, false) as Button)
	await _click(dialog.find_child("SettingsOK", true, false) as Button)
	_check(not is_instance_valid(_application.get("_settings_window")) and not _window.gui_embed_subwindows, "Public embedded OK applies after hiding the keyboard")
	print("BGR_MODAL_PROGRESS applied-six")
	_check(int(_simulation.settings_snapshot()["values"]["DELAY"]) == 6, "Keyboard edits use the original successful Parameters transaction")
	_keyboard.call("set_enabled", false)
	_check(_simulation.snapshot() == before, "Modal UI does not advance or mutate the biological snapshot")
	var after_file: String = OS.get_user_data_dir().path_join("modal-after.bgw")
	_check(bool(_simulation.save_world_file(after_file)["ok"]), "Save the complete final World and RNG")
	_check(FileAccess.get_file_as_bytes(before_file) == FileAccess.get_file_as_bytes(after_file), "Modal lifecycle preserves exact serialized World and RNG bytes")
	dialog = await _open(true)
	if not is_instance_valid(dialog):
		await _finish()
		return
	_application.call("shutdown", true)
	await _frames()
	_check(not is_instance_valid(dialog) and not _window.gui_embed_subwindows, "Main shutdown removes the modal and restores parent embedding")
	await _finish()
