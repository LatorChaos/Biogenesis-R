extends "res://ui/native_frame_loop.gd"

const Main = preload("res://ui/main.gd")
var _keyboard: Node
var _checks: int = 0
var _failures: Array[String] = []
var _submitted: int = 0
var _changed: int = 0
var _cancelled: int = 0
var _file_selected: int = 0
var _notified: Dictionary = {}
var _observations: Array[Dictionary] = []


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


func _click(control: Control) -> void:
	var window: Window = control.get_window()
	var position: Vector2 = control.get_global_rect().get_center()
	if not _notified.has(window.get_window_id()):
		window.notify_mouse_entered()
		_notified[window.get_window_id()] = true
	var motion: InputEventMouseMotion = InputEventMouseMotion.new()
	motion.window_id = window.get_window_id()
	motion.position = position
	motion.global_position = position
	motion.relative = Vector2.ONE
	window.push_input(motion, true)
	for down: bool in [true, false]:
		var event: InputEventMouseButton = InputEventMouseButton.new()
		event.window_id = window.get_window_id()
		event.position = position
		event.global_position = position
		event.button_index = MOUSE_BUTTON_LEFT
		event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
		event.pressed = down
		window.push_input(event, true)
		await process_frame
	await _frames()


func _panel(window: Window) -> Control:
	return window.find_child("TouchKeyboard", true, false) as Control


func _key(window: Window, name: String) -> void:
	var button: Button = _panel(window).find_child(name, true, false) as Button
	_check(button != null and button.is_visible_in_tree(), "Visible keyboard key: " + name)
	if button != null:
		var before_mode: String = str(_keyboard.get("_mode"))
		await _click(button)
		_observations.append({"key": name, "before_mode": before_mode, "after_mode": _keyboard.get("_mode"), "visible": _panel(window).visible if is_instance_valid(window) and _panel(window) != null else false})


func _letter(window: Window, value: String) -> void:
	await _key(window, "TouchCharacter" + str(value.unicode_at(0)))


func _native_key(window: Window, character: String) -> void:
	var button: Button = _panel(window).find_child("TouchCharacter" + str(character.unicode_at(0)), true, false) as Button
	for down: bool in [true, false]:
		var event: InputEventScreenTouch = InputEventScreenTouch.new()
		event.window_id = window.get_window_id()
		event.index = 0
		event.position = button.get_global_rect().get_center()
		event.pressed = down
		Input.parse_input_event(event)
		await process_frame
	await _frames()


func _capture(window: Window, name: String) -> void:
	await RenderingServer.frame_post_draw
	var image: Image = window.get_texture().get_image()
	_check(image.save_png(OS.get_environment("BGR_KEYBOARD_OUTPUT").path_join(name)) == OK, "Save rendered keyboard: " + name)


func _window(size: Vector2i) -> Window:
	var window: Window = Window.new()
	window.hide()
	window.force_native = true
	window.size = size
	root.add_child(window)
	return window


func _run() -> void:
	_keyboard = root.get_node("TouchKeyboard")
	_check(not DisplayServer.has_feature(DisplayServer.FEATURE_VIRTUAL_KEYBOARD), "Exercise the Linux fallback without native OSK support")
	_check(bool(_keyboard.call("is_enabled")), "Linux fallback defaults enabled without touchscreen detection")
	var simulation: BgrSimulation = BgrSimulation.new()
	_check(bool(simulation.preferences_initialize()["ok"]), "Initialize private production profile")
	var application: Main = Main.new()
	application.startup_seed_text = "827364"
	application.startup_dimensions = Vector2i(96, 96)
	application.startup_attempts = 0
	_check(application.initialize_session(simulation), "Use production Main with a process-disabled empty world")
	var main_window: Window = _window(Vector2i(1280, 800))
	main_window.add_child(application)
	application.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	application.set_process(false)
	main_window.popup_centered()
	await _frames()
	var shortcut: InputEventKey = InputEventKey.new()
	shortcut.window_id = main_window.get_window_id()
	shortcut.keycode = KEY_F10
	shortcut.pressed = true
	main_window.push_input(shortcut, true)
	await _frames()
	var before: Dictionary = simulation.snapshot()
	var canvas: Control = application.get("_canvas") as Control
	var full_rect: Rect2 = canvas.get_global_rect()
	var seed: LineEdit = application.find_child("SeedInput", true, false) as LineEdit
	await _click(seed)
	_check(_panel(main_window) != null and _panel(main_window).visible and main_window.gui_get_focus_owner() == seed, "Real production field focus reveals the keyboard without focus theft")
	await _key(main_window, "TouchSelectAll")
	await _key(main_window, "TouchMode")
	await _letter(main_window, "4")
	await _letter(main_window, "2")
	_check(seed.text == "42", "Numeric keys replace the actual selected seed text")
	await _key(main_window, "TouchBackspace")
	_check(seed.text == "4", "Backspace uses the editor action")
	await _letter(main_window, "8")
	await _key(main_window, "TouchLeft")
	await _key(main_window, "TouchBackspace")
	_check(seed.text == "8", "Arrow movement and backspace preserve caret semantics")
	await _capture(main_window, "production-numeric-1280.png")
	await _key(main_window, "TouchHide")
	_check(not _panel(main_window).visible and canvas.get_global_rect() == full_rect, "Hide restores the complete unchanged world viewport")
	await _click(seed)
	_check(_panel(main_window).visible, "Tap an already focused field to reopen the keyboard")
	_check(simulation.snapshot() == before, "Editing setup text executes no simulation command")
	var compact: Window = _window(Vector2i(640, 460))
	var line: LineEdit = LineEdit.new()
	line.position = Vector2(20, 210)
	line.size = Vector2(600, 48)
	line.max_length = 3
	line.text = "12"
	line.text_changed.connect(func(_text: String) -> void: _changed += 1)
	line.text_submitted.connect(func(_text: String) -> void: _submitted += 1)
	compact.add_child(line)
	compact.popup_centered()
	await _frames()
	await _click(line)
	await _key(compact, "TouchSelectAll")
	await _letter(compact, "p")
	await _key(compact, "TouchShift")
	await _letter(compact, "A")
	await _letter(compact, "B")
	await _letter(compact, "C")
	_check(line.text == "pAB" and _changed >= 3, "Original text_changed and max_length rules remain active")
	_check(not _panel(compact).get_global_rect().intersects(line.get_global_rect()), "Compact keyboard keeps the focused field visible")
	_check(Rect2(Vector2.ZERO, Vector2(compact.size)).encloses(_panel(compact).get_global_rect()), "Compact keyboard remains inside its native viewport")
	await _capture(compact, "compact-text-640.png")
	await _key(compact, "TouchEnter")
	_check(_submitted == 1 and not _panel(compact).visible, "Enter submits once through ordinary LineEdit input and hides")
	var resize_window: Window = _window(Vector2i(760, 580))
	var resize_edit: LineEdit = LineEdit.new()
	resize_edit.position = Vector2(20, 210)
	resize_edit.size = Vector2(600, 48)
	resize_edit.text = "resize"
	resize_window.add_child(resize_edit)
	resize_window.popup_centered()
	await _frames()
	await _click(resize_edit)
	await _key(resize_window, "TouchMode")
	await _key(resize_window, "TouchShift")
	var resize_text: String = resize_edit.text
	var resize_caret: int = resize_edit.caret_column
	var resize_key_id: int = (_panel(resize_window).find_child("TouchMode", true, false) as Button).get_instance_id()
	resize_window.size = Vector2i(780, 580)
	await _frames()
	_check((_panel(resize_window).find_child("TouchMode", true, false) as Button).get_instance_id() == resize_key_id,
		"Resize within the same key metrics does not rebuild keyboard buttons")
	resize_window.size = Vector2i(640, 460)
	await _frames()
	_check(Rect2(Vector2.ZERO, Vector2(resize_window.size)).encloses(_panel(resize_window).get_global_rect()),
		"Opened760 keyboard fits after native resize640 without refocus")
	_check(not _panel(resize_window).get_global_rect().intersects(resize_edit.get_global_rect()),
		"Resize640 keeps the same focused editor unobscured")
	_check(resize_edit.text == resize_text and resize_edit.caret_column == resize_caret and resize_edit.has_focus()
		and str(_keyboard.get("_mode")) == "numbers" and bool(_keyboard.get("_shift")),
		"Resize640 preserves text caret focus mode and Shift")
	await _capture(resize_window, "resized-keyboard-640.png")
	resize_window.size = Vector2i(760, 580)
	await _frames()
	_check(Rect2(Vector2.ZERO, Vector2(resize_window.size)).encloses(_panel(resize_window).get_global_rect())
		and resize_edit.has_focus() and resize_edit.text == resize_text and str(_keyboard.get("_mode")) == "numbers",
		"Native resize760 restores the layout without changing the active editor or mode")
	await _capture(resize_window, "restored-keyboard-760.png")
	await _key(resize_window, "TouchSelectAll")
	await _letter(resize_window, "7")
	_check(resize_edit.text == "7", "Keyboard remains usable after both native resizes")
	resize_window.queue_free()
	await _frames()
	compact.queue_free()
	await _frames()
	var multi_window: Window = _window(Vector2i(800, 660))
	var multi: TextEdit = TextEdit.new()
	multi.position = Vector2(20, 20)
	multi.size = Vector2(760, 240)
	multi_window.add_child(multi)
	multi_window.popup_centered()
	await _frames()
	await _click(multi)
	await _native_key(multi_window, "q")
	await _letter(multi_window, "w")
	await _key(multi_window, "TouchEnter")
	_check(multi.text == "qw\n" and _panel(multi_window).visible, "TextEdit Enter inserts a newline and retains its keyboard")
	await _key(multi_window, "TouchSelectAll")
	await _key(multi_window, "TouchBackspace")
	_check(multi.text.is_empty(), "TextEdit selection/backspace uses ordinary multiline editing")
	_keyboard.call("set_enabled", false)
	_check(not _panel(multi_window).visible, "Explicit user preference disables the fallback")
	_keyboard.call("set_enabled", true)
	await _frames()
	_check(_panel(multi_window).visible, "Re-enabling restores the focused editor")
	multi_window.queue_free()
	await _frames()
	var dialog: ConfirmationDialog = ConfirmationDialog.new()
	dialog.force_native = true
	dialog.exclusive = true
	root.add_child(dialog)
	var dialog_edit: LineEdit = LineEdit.new()
	dialog_edit.custom_minimum_size = Vector2(440, 48)
	dialog.add_child(dialog_edit)
	dialog_edit.position = Vector2(20, 150)
	dialog_edit.size = Vector2(600, 48)
	dialog.canceled.connect(func() -> void: _cancelled += 1)
	dialog.popup_centered(Vector2i(640, 460))
	await _frames()
	await _click(dialog_edit)
	_check(_panel(dialog).get_window() == dialog, "Keyboard shares the modal editor's own native Window")
	await _key(dialog, "TouchCancel")
	_check(_cancelled == 1 and not dialog.visible, "Cancel follows the original dialog Escape path")
	dialog.queue_free()
	await _frames()
	var picker: FileDialog = FileDialog.new()
	picker.force_native = true
	picker.file_mode = FileDialog.FILE_MODE_SAVE_FILE
	picker.access = FileDialog.ACCESS_FILESYSTEM
	picker.current_dir = OS.get_user_data_dir()
	root.add_child(picker)
	picker.file_selected.connect(func(_path: String) -> void: _file_selected += 1)
	picker.popup_centered(Vector2i(800, 560))
	await _frames()
	await _click(picker.get_line_edit())
	await _letter(picker, "a")
	await _letter(picker, "b")
	await _key(picker, "TouchEnter")
	_check(_file_selected == 1 and not picker.visible, "Filename Enter reaches ordinary FileDialog submission without creating a file")
	picker.queue_free()
	await _frames()
	var original_delay: int = int(simulation.settings_snapshot()["values"]["DELAY"])
	await _click(application.find_child("SettingsButton", true, false) as Button)
	var parameters: Window = application.get("_settings_window") as Window
	await _frames()
	var delay: LineEdit = parameters.find_child("Setting_DELAY", true, false) as LineEdit
	await _click(delay)
	_check(_panel(parameters).visible and not _panel(parameters).get_global_rect().intersects(delay.get_global_rect()), "Production parameter editor remains visible with its modal keyboard")
	await _key(parameters, "TouchSelectAll")
	await _key(parameters, "TouchMode")
	await _letter(parameters, "3")
	await _key(parameters, "TouchCancel")
	_check(int(simulation.settings_snapshot()["values"]["DELAY"]) == original_delay and not is_instance_valid(application.get("_settings_window")), "Keyboard Cancel follows production settings Escape and discards the draft")
	await _click(application.find_child("SettingsButton", true, false) as Button)
	parameters = application.get("_settings_window") as Window
	await _frames()
	delay = parameters.find_child("Setting_DELAY", true, false) as LineEdit
	await _click(delay)
	await _key(parameters, "TouchSelectAll")
	await _key(parameters, "TouchMode")
	await _letter(parameters, "4")
	await _capture(parameters, "production-settings-760.png")
	await _key(parameters, "TouchHide")
	await _click(parameters.find_child("SettingsOK", true, false) as Button)
	_check(int(simulation.settings_snapshot()["values"]["DELAY"]) == 4, "Touch editing and ordinary Settings OK preserve original apply/preferences semantics")
	main_window.queue_free()
	await _frames()
	print("BGR_TOUCH_KEYBOARD " + JSON.stringify({"status": "PASS" if _failures.is_empty() else "FAIL", "checks": _checks,
		"failures": _failures, "observations": _observations, "physical_touch_or_Pi": false, "route": "Public Window.push_input through real focused editors and Buttons",
		"native_virtual_keyboard_feature": DisplayServer.has_feature(DisplayServer.FEATURE_VIRTUAL_KEYBOARD)}))
	quit(0 if _failures.is_empty() else 1)
