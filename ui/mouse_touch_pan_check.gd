extends "res://ui/native_frame_loop.gd"

const Main = preload("res://ui/main.gd")

class RawObserver extends Node:
	var record: Callable

	func _input(event: InputEvent) -> void:
		if event is InputEventScreenTouch or event is InputEventScreenDrag or event is InputEventMouse:
			record.call("input", event)

var _window: Window
var _main: Main
var _canvas: Control
var _shell: Control
var _checks: int = 0
var _failures: Array[String] = []
var _picks: int = 0
var _events: Array[Dictionary] = []
var _cases: Array[Dictionary] = []
var _phase: String = "setup"
var _native_focus_verified: bool = false


func _initialize() -> void:
	_run.call_deferred()


func _check(value: bool, description: String) -> void:
	_checks += 1
	if not value:
		_failures.append(description)


func _record(stage: String, event: InputEvent) -> void:
	if _events.size() >= 200:
		return
	var row: Dictionary = {"phase": _phase, "stage": stage, "class": event.get_class(), "device": event.device}
	if event is InputEventScreenTouch:
		row.merge({"index": event.index, "pressed": event.pressed, "canceled": event.canceled})
	if event is InputEventScreenDrag:
		row["index"] = event.index
	_events.append(row)


func _send(event: InputEvent) -> void:
	(event as InputEventFromWindow).window_id = _window.get_window_id()
	_window.push_input(event, true)
	await process_frame


func _touch(index: int, down: bool, position: Vector2, canceled: bool = false, device: int = 0) -> void:
	var event: InputEventScreenTouch = InputEventScreenTouch.new()
	event.index = index
	event.position = position
	event.pressed = down
	event.canceled = canceled
	event.device = device
	await _send(event)


func _drag(index: int, position: Vector2, relative: Vector2) -> void:
	var event: InputEventScreenDrag = InputEventScreenDrag.new()
	event.index = index
	event.position = position
	event.relative = relative
	await _send(event)


func _mouse(down: bool, position: Vector2, device: int = 32, ctrl: bool = false, canceled: bool = false) -> void:
	var event: InputEventMouseButton = InputEventMouseButton.new()
	event.position = position
	event.global_position = position
	event.button_index = MOUSE_BUTTON_LEFT
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
	event.pressed = down
	event.device = device
	event.ctrl_pressed = ctrl
	event.canceled = canceled
	await _send(event)


func _motion(position: Vector2, relative: Vector2, down: bool, ctrl: bool = false) -> void:
	var event: InputEventMouseMotion = InputEventMouseMotion.new()
	event.position = position
	event.global_position = position
	event.relative = relative
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
	event.device = 32
	event.ctrl_pressed = ctrl
	await _send(event)


func _idle() -> void:
	await create_timer(0.65).timeout
	await process_frame
	_check(not (_shell.get("_hud") as Control).visible, _phase + ": HUD becomes hidden through real elapsed processing")


func _observe_case(before: Dictionary, picks: int) -> Dictionary:
	var row: Dictionary = {"name": _phase, "before": before, "after": _canvas.call("view_state"),
		"picks": _picks - picks, "HUD": (_shell.get("_hud") as Control).visible,
		"touch_contacts": (_canvas.get("_touches") as Dictionary).size()}
	_cases.append(row)
	return row


func _run() -> void:
	root.size = Vector2i(1280, 800)
	var simulation: BgrSimulation = BgrSimulation.new()
	_check(bool(simulation.preferences_initialize()["ok"]), "Initialize an independent fixture profile")
	_main = Main.new()
	_main.startup_seed_text = "827364"
	_main.startup_dimensions = Vector2i(7000, 7000)
	_main.startup_attempts = 0
	_check(_main.initialize_session(simulation), "Initialize the real Main/Rust fixture")
	_window = Window.new()
	_window.hide()
	_window.force_native = true
	_window.size = Vector2i(1280, 800)
	root.add_child(_window)
	_window.add_child(_main)
	_main.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_main.set_process(false)
	_canvas = _main.get("_canvas") as Control
	_shell = _main.get("_ambient_shell") as Control
	_shell.set("idle_seconds", 0.05)
	_canvas.connect("organism_picked", func(_a: Vector2, _b: bool, _c: Vector2) -> void: _picks += 1)
	_canvas.gui_input.connect(func(event: InputEvent) -> void: _record("gui", event))
	var observer: RawObserver = RawObserver.new()
	observer.record = _record
	_window.add_child(observer)
	_window.popup_centered()
	await process_frame
	await process_frame
	_window.notify_mouse_entered()
	_check(_canvas.get_global_rect().size == Vector2(1280, 800), "Production Canvas occupies the native child viewport")
	var state: Dictionary = simulation.snapshot()
	var before_save: String = OS.get_user_data_dir().path_join("mouse-before.bgw")
	_check(bool(simulation.save_world_file(before_save)["ok"]), "Save the initial complete World and RNG fixture")
	_phase = "hidden_native_wake"
	await _idle()
	var before: Dictionary = _canvas.call("view_state")
	var picks: int = _picks
	await _touch(0, true, Vector2(500, 350))
	await _touch(0, false, Vector2(500, 350))
	var row: Dictionary = _observe_case(before, picks)
	_check(row["HUD"] and row["picks"] == 0 and row["after"] == before, "Hidden native touch wakes only")
	_phase = "visible_native_pan"
	before = _canvas.call("view_state")
	picks = _picks
	await _touch(1, true, Vector2(500, 350))
	await _drag(1, Vector2(520, 360), Vector2(20, 10))
	await _drag(1, Vector2(535, 370), Vector2(15, 10))
	await _touch(1, false, Vector2(535, 370))
	row = _observe_case(before, picks)
	_check(row["after"]["center"] != before["center"] and row["after"]["zoom"] == before["zoom"] and row["picks"] == 0, "Native pan passes full Main/Shell GUI propagation")
	_phase = "visible_native_pinch"
	before = _canvas.call("view_state")
	picks = _picks
	await _touch(2, true, Vector2(450, 350))
	await _touch(3, true, Vector2(550, 350))
	await _drag(3, Vector2(590, 350), Vector2(40, 0))
	await _touch(2, false, Vector2(450, 350))
	await _touch(3, false, Vector2(590, 350))
	row = _observe_case(before, picks)
	_check(row["after"]["zoom"] > before["zoom"] and row["picks"] == 0 and row["touch_contacts"] == 0, "Native pinch passes full Main/Shell GUI propagation")
	_phase = "native_cancel"
	before = _canvas.call("view_state")
	picks = _picks
	await _touch(4, true, Vector2(500, 350))
	await _touch(4, false, Vector2(500, 350), true)
	await _drag(4, Vector2(530, 350), Vector2(30, 0))
	await _touch(4, false, Vector2(530, 350))
	row = _observe_case(before, picks)
	_check(row["after"] == before and row["picks"] == 0 and row["touch_contacts"] == 0, "Canceled contact cannot pan or select through full route")
	_phase = "hidden_native_gesture_wakes_only"
	await _idle()
	before = _canvas.call("view_state")
	picks = _picks
	await _touch(5, true, Vector2(500, 350))
	await _drag(5, Vector2(540, 350), Vector2(40, 0))
	await _touch(5, false, Vector2(540, 350))
	row = _observe_case(before, picks)
	_check(row["after"] == before and row["picks"] == 0 and row["touch_contacts"] == 0, "Hidden gesture has no partial contact in Canvas")
	_phase = "ordinary_mouse_left_drag"
	before = _canvas.call("view_state")
	picks = _picks
	await _motion(Vector2(500, 350), Vector2(1, 0), false)
	await _mouse(true, Vector2(500, 350))
	await _motion(Vector2(540, 350), Vector2(40, 0), true)
	await _mouse(false, Vector2(540, 350))
	row = _observe_case(before, picks)
	_check(row["after"]["center"] != before["center"] and row["picks"] == 0, "Ordinary device32 left drag pans without selection")
	_phase = "ctrl_left_drag"
	before = _canvas.call("view_state")
	picks = _picks
	await _mouse(true, Vector2(500, 350), 32, true)
	await _motion(Vector2(540, 350), Vector2(40, 0), true, true)
	await _mouse(false, Vector2(540, 350), 32, true)
	row = _observe_case(before, picks)
	_check(row["after"] == before and row["picks"] == 1, "Ctrl-left retains legacy release selection without pan")
	_phase = "left_jitter"
	before = _canvas.call("view_state")
	picks = _picks
	await _mouse(true, Vector2(500, 350))
	await _motion(Vector2(502, 351), Vector2(2, 1), true)
	await _mouse(false, Vector2(502, 351))
	row = _observe_case(before, picks)
	_check(row["after"] == before and row["picks"] == 1, "Subthreshold left jitter clicks exactly once")
	_phase = "left_release_without_motion"
	picks = _picks
	await _mouse(true, Vector2(500, 350))
	await _mouse(false, Vector2(540, 350))
	_check(_picks == picks, "Distant release without retained motion must not select")
	_phase = "mouse_cancel"
	before = _canvas.call("view_state")
	picks = _picks
	await _mouse(true, Vector2(500, 350))
	await _mouse(false, Vector2(500, 350), 32, false, true)
	await _motion(Vector2(530, 350), Vector2(30, 0), true)
	await _mouse(false, Vector2(530, 350))
	_check(_canvas.call("view_state") == before and _picks == picks, "Canceled mouse remainder neither pans nor selects")
	_phase = "lost_mouse_button"
	before = _canvas.call("view_state")
	picks = _picks
	await _mouse(true, Vector2(500, 350))
	await _motion(Vector2(530, 350), Vector2(30, 0), false)
	await _mouse(false, Vector2(530, 350))
	_check(_canvas.call("view_state") == before and _picks == picks, "Missing left mask cancels pending click and pan")
	_phase = "focus_loss"
	_window.grab_focus()
	await process_frame
	await _mouse(true, Vector2(500, 350))
	before = _canvas.call("view_state")
	picks = _picks
	var sink: Window = Window.new()
	sink.hide()
	sink.force_native = true
	sink.size = Vector2i(120, 100)
	root.add_child(sink)
	sink.popup_centered()
	sink.grab_focus()
	await process_frame
	await process_frame
	_native_focus_verified = not _window.has_focus()
	_window.grab_focus()
	await process_frame
	await process_frame
	await _motion(Vector2(540, 350), Vector2(40, 0), true)
	await _mouse(false, Vector2(540, 350))
	if _native_focus_verified:
		_check(_canvas.call("view_state") == before and _picks == picks, "Actual focus loss clears pending mouse gesture")
	sink.queue_free()
	_phase = "hidden_motion_then_mouse_tap"
	await _idle()
	before = _canvas.call("view_state")
	picks = _picks
	await _motion(Vector2(500, 350), Vector2(1, 0), false)
	await _mouse(true, Vector2(500, 350))
	await _mouse(false, Vector2(500, 350))
	row = _observe_case(before, picks)
	_check(row["picks"] == 0, "First tap after motion woke hidden HUD must not select")
	_phase = "engine_emulated_mouse_pair"
	before = _canvas.call("view_state")
	picks = _picks
	await _mouse(true, Vector2(500, 350), InputEvent.DEVICE_ID_EMULATION)
	await _mouse(false, Vector2(500, 350), InputEvent.DEVICE_ID_EMULATION)
	row = _observe_case(before, picks)
	_check(row["after"] == before and row["picks"] == 0, "Engine-emulated mouse alone does not duplicate Canvas actions")
	_phase = "motion_only_idle"
	await _idle()
	await _motion(Vector2(510, 350), Vector2(10, 0), false)
	await _idle()
	_check(simulation.snapshot() == state, "Full route does not advance or mutate biological fixture")
	var after_save: String = OS.get_user_data_dir().path_join("mouse-after.bgw")
	_check(bool(simulation.save_world_file(after_save)["ok"]), "Save the final complete World and RNG fixture")
	_check(FileAccess.get_file_as_bytes(before_save) == FileAccess.get_file_as_bytes(after_save), "Mouse and native gestures preserve serialized World and RNG bytes")
	var opened: Dictionary = simulation.open_laboratory("empty")
	var draft: BgrGenomeDraft = opened["draft"]
	_check(bool(draft.edit_rows("insert", 0)["ok"]), "Prepare an actual clipboard genotype")
	_check(bool(_main.call("_accept_result", simulation.commit_laboratory(draft))), "Accept real clipboard state for placement fixture")
	_main.call("_set_placement_mode", "paste")
	_shell.call("reveal")
	picks = _picks
	await _mouse(true, Vector2(700, 400), 32, true)
	await _mouse(false, Vector2(700, 400), 32, true)
	_check(_picks == picks + 1 and int(simulation.snapshot()["population"]) == 1 and str(_main.get("_placement_mode")).is_empty(), "Ctrl-click pastes exactly once through unchanged Main placement callback")
	print("BGR_MOUSE_TOUCH_PAN " + JSON.stringify({"status": "PASS" if _failures.is_empty() else "FAIL", "checks": _checks,
		"failures": _failures, "cases": _cases, "events": _events, "physical_touch_claim": false, "native_focus_transfer_verified": _native_focus_verified,
		"route": "Public native child Window.push_input(local=true), unchanged production Main/Shell/Canvas",
		"fixture": "Empty7000x7000, paused and process-disabled; public native Window input; actual private focus change"}))
	_main.queue_free()
	await process_frame
	_window.queue_free()
	await process_frame
	quit(0 if _failures.is_empty() else 1)
