extends SceneTree

const Shell = preload("res://ui/ambient_shell.gd")

class ClockShell extends Shell:
	var now_usec: int = 0

	func _clock_usec() -> int:
		return now_usec

class CanvasProbe extends Control:
	var clicks: int = 0

	func _gui_input(event: InputEvent) -> void:
		if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
			clicks += 1
			accept_event()

var _checks: int = 0
var _failures: Array[String] = []
var _pause_requests: int = 0
var _cinema_requests: int = 0
var _native_cursor_hidden: bool = false
var _shell: ClockShell


func _initialize() -> void:
	_run.call_deferred()


func _check(condition: bool, description: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(description)


func _run() -> void:
	var initial_mouse_mode: Input.MouseMode = Input.mouse_mode
	root.size = Vector2i(1024, 768)
	var host: Control = Control.new()
	root.add_child(host)
	host.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var tools: VBoxContainer = VBoxContainer.new()
	tools.name = "LegacyTools"
	host.add_child(tools)
	var retained: Button = Button.new()
	retained.name = "RetainedAction"
	retained.text = "Retained action"
	retained.custom_minimum_size.y = 1200.0
	tools.add_child(retained)
	var canvas: CanvasProbe = CanvasProbe.new()
	canvas.name = "WorldCanvas"
	tools.add_child(canvas)
	var inspector: VBoxContainer = VBoxContainer.new()
	inspector.name = "OrganismControls"
	tools.add_child(inspector)
	inspector.hide()
	_shell = ClockShell.new()
	host.add_child(_shell)
	_shell.configure(canvas, tools, inspector)
	_shell.set_process(false)
	_shell.pause_requested.connect(func() -> void: _pause_requests += 1)
	_shell.cinema_toggled.connect(func(_enabled: bool) -> void: _cinema_requests += 1)
	await process_frame
	await process_frame
	_check(canvas.get_global_rect().is_equal_approx(_shell.get_global_rect()), "Canvas fills the shell without permanent chrome")
	_check(retained.get_parent() == tools and retained.name == "RetainedAction", "Keep the existing tools and action identities")
	var before: Rect2 = canvas.get_global_rect()
	_shell.show_tools()
	await process_frame
	_check((_shell.find_child("AmbientToolsDrawer", true, false) as Control).visible, "Open the retained tools drawer")
	var tools_scroll: ScrollContainer = _shell.find_child("AmbientToolsScroll", true, false) as ScrollContainer
	_check(tools_scroll.get_v_scroll_bar().max_value > tools_scroll.get_v_scroll_bar().page, "Retained tools can scroll beyond the drawer height")
	_check(canvas.get_global_rect().is_equal_approx(before), "Opening the drawer does not resize the canvas")
	_shell.toggle_tools()
	inspector.show()
	_shell.reveal()
	await process_frame
	var inspector_panel: Control = _shell.find_child("AmbientInspector", true, false) as Control
	_check(inspector_panel.visible, "Show the selected organism in a floating inspector")
	(_shell.find_child("AmbientCloseInspector", true, false) as Button).pressed.emit()
	_check(not inspector_panel.visible and canvas.get_global_rect().is_equal_approx(before), "Dismiss the inspector without resizing the canvas")
	_shell.reveal()
	_check(inspector_panel.visible, "Reveal the selected inspector again")
	inspector.hide()
	var pause: Button = _shell.find_child("AmbientPause", true, false) as Button
	for language: String in ["en", "es", "ca", "de", "pt", "ru"]:
		_shell.set_language(language)
		_shell.set_paused(true)
		_check(not pause.text.is_empty() and pause.icon == Shell.RESUME_ICON, "Localize Resume and retain its icon: " + language)
		_shell.set_paused(false)
		_check(not pause.text.is_empty() and pause.icon == Shell.PAUSE_ICON, "Localize Pause and retain its icon: " + language)
	_shell.set_language("en")
	await process_frame
	for node_name: String in ["AmbientPause", "AmbientTools", "AmbientFit", "AmbientCinema"]:
		var button: Button = _shell.find_child(node_name, true, false) as Button
		_check(button.size.x >= 48.0 and button.size.y >= 48.0 and button.focus_mode == Control.FOCUS_ALL, "Keep a keyboard-accessible touch target: " + node_name)
	await _click(pause.get_global_rect().get_center())
	_check(_pause_requests == 1, "The real GUI pause button emits one request")
	_shell.reveal()
	for unused: int in range(22):
		_idle_step(0.25)
	var hud: Control = _shell.find_child("AmbientHud", true, false) as Control
	_check(not hud.visible and hud.mouse_behavior_recursive == Control.MOUSE_BEHAVIOR_DISABLED, "The idle HUD fully hides and disables input")
	var can_hide_cursor: bool = DisplayServer.has_feature(DisplayServer.FEATURE_MOUSE) and root.has_focus() and initial_mouse_mode == Input.MOUSE_MODE_VISIBLE
	_check(Input.mouse_mode == (Input.MOUSE_MODE_HIDDEN if can_hide_cursor else initial_mouse_mode),
		"Hide the focused native cursor only when its backend supports a mouse")
	_native_cursor_hidden = can_hide_cursor and Input.mouse_mode == Input.MOUSE_MODE_HIDDEN
	_check(canvas.modulate == Color.WHITE, "Fading chrome does not modulate the world")
	await _click(canvas.get_global_rect().get_center())
	_check(hud.visible and canvas.clicks == 0, "The first hidden-HUD click wakes controls without selecting the world")
	_check(Input.mouse_mode == initial_mouse_mode, "Activity restores a cursor hidden by this shell")
	await _click(canvas.get_global_rect().get_center())
	_check(canvas.clicks == 1, "Later world clicks reach the canvas")
	_shell.hold_open = true
	for unused: int in range(24):
		_idle_step(0.25)
	_check(hud.visible, "The owner can hold controls open during errors or modal operations")
	_shell.hold_open = false
	root.size = Vector2i(800, 660)
	await process_frame
	await process_frame
	_check(canvas.get_global_rect().is_equal_approx(_shell.get_global_rect()), "Canvas continues to fill the resized window")
	_shell.reveal()
	for unused: int in range(22):
		_idle_step(0.25)
	var touch: InputEventScreenTouch = InputEventScreenTouch.new()
	touch.position = Vector2(20.0, 20.0)
	touch.pressed = true
	Input.parse_input_event(touch)
	await process_frame
	touch = InputEventScreenTouch.new()
	touch.position = Vector2(20.0, 20.0)
	touch.pressed = false
	Input.parse_input_event(touch)
	await process_frame
	_check(hud.visible and canvas.clicks == 1, "Hidden-HUD touch wakes controls without selecting the world")
	_shell.set_cinema_enabled(true)
	_check((_shell.find_child("AmbientCinema", true, false) as Button).button_pressed and _cinema_requests == 0,
		"Restoring the cinema preference changes its button without emitting an action")
	_shell.idle_seconds = 1.0
	_shell.reveal()
	for unused: int in range(6):
		_idle_step(0.25)
	_check(not hud.visible, "Use the configured HUD idle duration")
	_shell.idle_seconds = 5.0
	_shell.show_tools()
	for unused: int in range(22):
		_idle_step(0.25)
	_check(hud.visible, "Keep an active tools drawer available during inactivity")
	_shell.toggle_tools()
	var tools_button: Button = _shell.find_child("AmbientTools", true, false) as Button
	tools_button.grab_focus()
	var pause_key: InputEventKey = InputEventKey.new()
	pause_key.keycode = KEY_P
	pause_key.pressed = true
	_shell._input(pause_key)
	for unused: int in range(22):
		_idle_step(0.25)
	_check(not hud.visible and hud.mouse_behavior_recursive == Control.MOUSE_BEHAVIOR_DISABLED and _shell.is_idle(),
		"A focused dock button and P activity do not prevent complete idle fading")
	root.gui_release_focus()
	var tab_key: InputEventKey = InputEventKey.new()
	tab_key.keycode = KEY_TAB
	tab_key.pressed = true
	_shell._input(tab_key)
	_check(hud.visible and root.gui_get_focus_owner() == tools_button,
		"The first hidden Tab wakes the dock with visible keyboard focus")
	_shell.reveal()
	_idle_step(_shell.idle_seconds + 0.5)
	_check(not hud.visible and _shell.is_idle(), "Elapsed idle time completes fading")
	_shell.reveal()
	_shell.now_usec += 5_500_000
	_shell._process(0.01)
	_check(not hud.visible and _shell.is_idle(), "Monotonic inactivity expires even when engine frame delta is capped")
	var editor: LineEdit = LineEdit.new()
	inspector.add_child(editor)
	inspector.show()
	_shell.reveal()
	editor.grab_focus()
	for unused: int in range(22):
		_idle_step(0.25)
	_check(hud.visible and root.gui_get_focus_owner() == editor, "Retain the HUD while a real text field is being edited")
	editor.release_focus()
	inspector.hide()
	for unused: int in range(22):
		_idle_step(0.25)
	_check(not hud.visible, "Resume complete idle fading after text editing ends")
	_shell.notification(Node.NOTIFICATION_WM_WINDOW_FOCUS_OUT)
	_check(Input.mouse_mode == initial_mouse_mode, "Focus loss restores only this shell's cursor change")
	print(JSON.stringify({"status": "PASS" if _failures.is_empty() else "FAIL", "checks": _checks, "failures": _failures,
		"native_cursor_hide_verified": _native_cursor_hidden, "display_backend": DisplayServer.get_name()}))
	host.queue_free()
	quit(0 if _failures.is_empty() else 1)


func _idle_step(seconds: float) -> void:
	_shell.now_usec += int(seconds * 1000000.0)
	_shell._process(seconds)


func _click(position: Vector2, move_pointer: bool = true) -> void:
	if move_pointer:
		var motion: InputEventMouseMotion = InputEventMouseMotion.new()
		motion.position = position
		motion.global_position = position
		Input.parse_input_event(motion)
	for pressed: bool in [true, false]:
		var event: InputEventMouseButton = InputEventMouseButton.new()
		event.position = position
		event.global_position = position
		event.button_index = MOUSE_BUTTON_LEFT
		event.button_mask = MOUSE_BUTTON_MASK_LEFT if pressed else 0
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame
