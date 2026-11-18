extends SceneTree

const Main = preload("res://ui/main.gd")
var _main: Control
var _canvas: Control
var _checks: int = 0


func _initialize() -> void:
	await process_frame
	root.size = Vector2i(1180, 820)
	var session: BgrSimulation = BgrSimulation.new()
	session.preferences_initialize()
	_main = Main.new()
	_main.set("startup_dimensions", Vector2i(7000, 7000))
	_main.set("startup_attempts", 0)
	_main.set("startup_seed_text", "827364")
	if not _check(_main.call("initialize_session", session), "Bind the actual session"):
		return
	root.add_child(_main)
	_main.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_main.set_process(false)
	await process_frame
	_canvas = _main.get("_canvas") as Control
	var saved: Dictionary = session.snapshot()
	var save_path: String = OS.get_environment("BGR_TEST_ROOT").path_join("navigation.bgw")
	if not _check(session.save_world_file(save_path)["ok"], "Save the actual world before navigation"):
		return
	var world_bytes: PackedByteArray = FileAccess.get_file_as_bytes(save_path)
	_canvas.call("zoom_at", 1.0 / float(_canvas.get("_zoom")), _canvas.size * 0.5)
	_canvas.grab_focus()
	await process_frame
	var origin: Vector2 = _canvas.get("_offset")
	await _key(KEY_RIGHT)
	await process_frame
	if not _check(_canvas.get("_offset") == origin - Vector2(10, 0) and root.gui_get_focus_owner() == _canvas, "Right scrolls ten pixels and keeps canvas focus"):
		return
	await _key(KEY_DOWN)
	if not _check(_canvas.get("_offset") == origin - Vector2(10, 10), "Down scrolls ten pixels"):
		return
	await _key(KEY_LEFT)
	await _key(KEY_UP)
	if not _check(_canvas.get("_offset") == origin, "Reverse arrows restore the view"):
		return
	await _key(KEY_RIGHT, true)
	if not _check(_canvas.get("_offset") == origin - Vector2(10, 0), "Held-key repeats scroll"):
		return
	await _key(KEY_RIGHT, false, false)
	if not _check(_canvas.get("_offset") == origin - Vector2(10, 0), "Key release does not scroll"):
		return
	for modifier: String in ["shift_pressed", "ctrl_pressed", "alt_pressed", "meta_pressed"]:
		_canvas.grab_focus()
		await _key(KEY_RIGHT, false, true, modifier)
		if not _check(_canvas.get("_offset") == origin - Vector2(10, 0), "Modified arrows retain their existing meaning: " + modifier):
			return
	_canvas.grab_focus()
	_canvas.set("_offset", Vector2(-5, -5))
	await _key(KEY_LEFT)
	await _key(KEY_UP)
	if not _check(_canvas.get("_offset") == Vector2.ZERO, "Clamp to the world's top and left edges"):
		return
	var far_edge: Vector2 = _canvas.size - Vector2(7000, 7000)
	_canvas.set("_offset", far_edge + Vector2(5, 5))
	await _key(KEY_RIGHT)
	await _key(KEY_DOWN)
	if not _check(_canvas.get("_offset") == far_edge, "Clamp to the world's bottom and right edges"):
		return
	_canvas.call("zoom_at", 2.0, _canvas.size * 0.5)
	origin = _canvas.get("_offset")
	await _key(KEY_LEFT)
	if not _check(_canvas.get("_offset") == origin + Vector2(10, 0), "Use ten screen pixels at double zoom"):
		return
	var seed: LineEdit = _main.get("_seed_input") as LineEdit
	seed.grab_focus()
	await process_frame
	seed.deselect()
	seed.caret_column = seed.text.length()
	await _key(KEY_LEFT)
	if not _check(_canvas.get("_offset") == origin + Vector2(10, 0) and seed.caret_column == seed.text.length() - 1, "Text-field arrows edit without panning: caret=%s length=%s focus=%s offset=%s expected=%s" % [seed.caret_column, seed.text.length(), root.gui_get_focus_owner(), _canvas.get("_offset"), origin + Vector2(10, 0)]):
		return
	_canvas.grab_focus()
	_canvas.call("fit_world")
	origin = _canvas.get("_offset")
	for code: int in [KEY_LEFT, KEY_RIGHT, KEY_UP, KEY_DOWN]:
		await _key(code)
	if not _check(_canvas.get("_offset") == origin and _canvas.get("_fit_enabled"), "A fitted world stays centered and keeps resize fitting"):
		return
	if not _check(session.snapshot() == saved, "Navigation leaves clock, selection and clipboard state unchanged"):
		return
	if not _check(session.save_world_file(save_path)["ok"] and FileAccess.get_file_as_bytes(save_path) == world_bytes, "Navigation preserves serialized world and RNG bytes"):
		return
	_main.queue_free()
	await process_frame
	print("KEYBOARD_NAVIGATION_PASS checks=", _checks)
	quit(0)


func _key(code: int, echo: bool = false, pressed: bool = true, modifier: String = "") -> void:
	var event: InputEventKey = InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	event.echo = echo
	if not modifier.is_empty():
		event.set(modifier, true)
	Input.parse_input_event(event)
	await process_frame
	if pressed:
		var release: InputEventKey = event.duplicate() as InputEventKey
		release.pressed = false
		release.echo = false
		Input.parse_input_event(release)
		await process_frame


func _check(value: bool, description: String) -> bool:
	_checks += 1
	if not value:
		printerr("KEYBOARD_NAVIGATION_FAIL: ", description)
		quit(1)
	return value
