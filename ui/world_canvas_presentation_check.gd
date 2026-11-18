extends SceneTree

const WorldCanvas = preload("res://ui/world_canvas.gd")

var _canvas: WorldCanvas
var _checks: int = 0
var _failures: Array[String] = []
var _picks: Array[Dictionary] = []
var _manual_changes: int = 0


func _initialize() -> void:
	_run.call_deferred()


func _check(condition: bool, message: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(message)


func _picked(position: Vector2, context_menu: bool, viewport_position: Vector2) -> void:
	_picks.append({"position": position, "context": context_menu, "viewport_position": viewport_position})


func _manual() -> void:
	_manual_changes += 1


func _touch(index: int, pressed: bool, position: Vector2, device: int = 0, canceled: bool = false) -> void:
	var event: InputEventScreenTouch = InputEventScreenTouch.new()
	event.index = index
	event.pressed = pressed
	event.position = position
	event.device = device
	event.canceled = canceled
	_canvas._gui_input(event)


func _drag(index: int, position: Vector2) -> void:
	var event: InputEventScreenDrag = InputEventScreenDrag.new()
	event.index = index
	event.position = position
	_canvas._gui_input(event)


func _mouse(button: int, pressed: bool, position: Vector2, device: int = InputEvent.DEVICE_ID_MOUSE) -> void:
	var event: InputEventMouseButton = InputEventMouseButton.new()
	event.button_index = button
	event.pressed = pressed
	event.position = position
	event.global_position = _canvas.get_global_transform_with_canvas() * position
	event.device = device
	_canvas._gui_input(event)


func _run() -> void:
	_canvas = WorldCanvas.new()
	_canvas.focus_mode = Control.FOCUS_ALL
	root.add_child(_canvas)
	_canvas.position = Vector2(17.0, 23.0)
	_canvas.size = Vector2(400.0, 300.0)
	_canvas.set_snapshot(PackedFloat64Array(), PackedInt32Array(), 1000.0, 1000.0)
	_canvas.organism_picked.connect(_picked)
	_canvas.manual_view_changed.connect(_manual)
	_canvas.set_camera_view(Vector2(250.0, 350.0), 2.0)
	var view: Dictionary = _canvas.view_state()
	_check(view["center"] == Vector2(250.0, 350.0) and view["zoom"] == 2.0 and view["world_size"] == Vector2(1000.0, 1000.0), "Camera view roundtrip differs")
	_check((_canvas.native_view_offset() + Vector2(250.0, 350.0) * 2.0) == _canvas.size * 0.5, "Camera center transform differs")
	view["center"] = Vector2.ZERO
	_check(_canvas.view_state()["center"] == Vector2(250.0, 350.0), "Returned view state aliases the camera")
	var before: Dictionary = _canvas.view_state()
	for center: Vector2 in [Vector2(NAN, 0.0), Vector2(0.0, INF), Vector2(-1.0, 0.0), Vector2(1001.0, 0.0)]:
		_canvas.set_camera_view(center, 1.0)
		_check(_canvas.view_state() == before, "Invalid camera center changed the view")
	for zoom: float in [0.0, -1.0, NAN, INF, WorldCanvas.MIN_ZOOM * 0.5, WorldCanvas.MAX_ZOOM * 2.0]:
		_canvas.set_camera_view(Vector2(500.0, 500.0), zoom)
		_check(_canvas.view_state() == before, "Invalid camera zoom changed the view")
	_canvas.set_selection({"bounds": PackedInt32Array([10, 20, 3, 4]), "infected": false})
	_canvas.follow_organism(PackedFloat64Array([500.0, 500.0]), 5.0)
	_canvas.fit_world()
	_canvas.zoom_at(1.2, _canvas.size * 0.5)
	_check(_manual_changes == 0 and _picks.is_empty(), "Automated camera/follow/fit emitted manual input or selection")
	_canvas.set_camera_view(Vector2(500.0, 500.0), 1.0)
	before = _canvas.view_state()
	_drag(99, Vector2(100.0, 100.0))
	_touch(99, false, Vector2(100.0, 100.0))
	_check(_canvas.view_state() == before and _picks.is_empty() and _manual_changes == 0, "Unmatched wake-only remainder changed the canvas")
	_touch(0, true, Vector2(100.0, 100.0))
	_mouse(MOUSE_BUTTON_LEFT, true, Vector2(100.0, 100.0), InputEvent.DEVICE_ID_EMULATION)
	_mouse(MOUSE_BUTTON_LEFT, false, Vector2(100.0, 100.0), InputEvent.DEVICE_ID_EMULATION)
	_touch(0, false, Vector2(100.0, 100.0))
	_check(_picks.size() == 1, "Native touch plus emulated mouse selected more than once")
	if _picks.size() == 1:
		_check(_picks[0]["position"] == Vector2(400.0, 450.0) and not _picks[0]["context"], "Touch selection transform differs")
		_check(_picks[0]["viewport_position"] == _canvas.get_global_transform_with_canvas() * Vector2(100.0, 100.0), "Touch popup anchor is not in viewport coordinates")
	_touch(0, true, Vector2(100.0, 100.0), InputEvent.DEVICE_ID_EMULATION)
	_touch(0, false, Vector2(100.0, 100.0), InputEvent.DEVICE_ID_EMULATION)
	_check(_picks.size() == 1, "Emulated touch duplicated physical mouse handling")
	before = _canvas.view_state()
	_touch(0, true, Vector2(50.0, 50.0))
	_drag(0, Vector2(52.0, 51.0))
	_check(_canvas.view_state() == before, "Touch jitter panned before the drag threshold")
	var offset: Vector2 = _canvas.native_view_offset()
	_drag(0, Vector2(60.0, 50.0))
	_check(_canvas.native_view_offset() == offset + Vector2(10.0, 0.0), "Single-finger pan lost the initial drag displacement")
	_touch(0, false, Vector2(60.0, 50.0))
	_check(_picks.size() == 1, "Single-finger pan also selected an organism")
	_canvas.set_camera_view(Vector2(500.0, 500.0), 1.0)
	_touch(0, true, Vector2(100.0, 100.0))
	_touch(1, true, Vector2(200.0, 100.0))
	_drag(1, Vector2(300.0, 100.0))
	_check(is_equal_approx(float(_canvas.view_state()["zoom"]), 2.0), "Two-finger distance did not set pinch scale")
	_check(_canvas.native_view_offset().is_equal_approx(Vector2(-700.0, -800.0)), "Pinch did not preserve the old centroid world anchor while panning")
	_touch(1, false, Vector2(300.0, 100.0))
	offset = _canvas.native_view_offset()
	_drag(0, Vector2(110.0, 110.0))
	_check(_canvas.native_view_offset().is_equal_approx(offset + Vector2(10.0, 10.0)), "Remaining pinch finger jumped to the original press")
	_touch(0, false, Vector2(110.0, 110.0))
	_check(_picks.size() == 1, "Pinch release also selected an organism")
	_touch(0, true, Vector2(100.0, 100.0))
	_touch(0, false, Vector2(100.0, 100.0), 0, true)
	_touch(0, false, Vector2(100.0, 100.0))
	_check(_picks.size() == 1, "Canceled touch selected an organism")
	_touch(0, true, Vector2(100.0, 100.0))
	_canvas._notification(Control.NOTIFICATION_WM_WINDOW_FOCUS_OUT)
	before = _canvas.view_state()
	_drag(0, Vector2(200.0, 200.0))
	_touch(0, false, Vector2(100.0, 100.0))
	_check(_canvas.view_state() == before and _picks.size() == 1, "Focus-loss remainder panned or selected")
	_touch(0, true, Vector2(NAN, 0.0))
	_drag(0, Vector2(INF, 0.0))
	_check(_canvas.view_state() == before, "Nonfinite touch changed the camera")
	_mouse(MOUSE_BUTTON_RIGHT, true, Vector2(100.0, 100.0))
	_mouse(MOUSE_BUTTON_RIGHT, false, Vector2(100.0, 100.0))
	_check(_picks.size() == 2 and _picks[-1]["context"], "Physical right-click context selection changed")
	_mouse(MOUSE_BUTTON_MIDDLE, true, Vector2(100.0, 100.0))
	var motion: InputEventMouseMotion = InputEventMouseMotion.new()
	motion.position = Vector2(120.0, 100.0)
	motion.relative = Vector2(20.0, 0.0)
	motion.device = InputEvent.DEVICE_ID_MOUSE
	offset = _canvas.native_view_offset()
	_canvas._gui_input(motion)
	_mouse(MOUSE_BUTTON_MIDDLE, false, Vector2(120.0, 100.0))
	_check(_canvas.native_view_offset() == offset + Vector2(20.0, 0.0) and _picks.size() == 2, "Middle-button pan changed or selected")
	var zoom_before: float = float(_canvas.view_state()["zoom"])
	_mouse(MOUSE_BUTTON_WHEEL_UP, true, Vector2(100.0, 100.0))
	_check(float(_canvas.view_state()["zoom"]) > zoom_before, "Physical wheel zoom stopped working")
	_check(_manual_changes > 0, "Real view and selection input did not emit manual_view_changed")
	_canvas.free()
	print("BGR_WORLD_CANVAS_PRESENTATION " + JSON.stringify({"result": "PASS" if _failures.is_empty() else "FAIL", "checks": _checks, "failures": _failures, "scope": "presentation transforms and direct native input objects; no physical touch or full gameplay claim"}))
	quit(0 if _failures.is_empty() else 1)
