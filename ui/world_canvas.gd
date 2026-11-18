extends Control

signal view_changed(zoom: float)
signal manual_view_changed()
signal organism_picked(position: Vector2, context_menu: bool, viewport_position: Vector2)

const MIN_ZOOM: float = 0.000000000001
const MAX_ZOOM: float = 1000000.0
const UNDERWATER_MATERIAL: ShaderMaterial = preload("res://ui/assets/underwater.tres")

var underwater_background: bool = false
var _network_presentation: PackedFloat64Array = PackedFloat64Array()
var _segments: PackedFloat64Array = PackedFloat64Array()
var _colors: PackedInt32Array = PackedInt32Array()
var _alphas: PackedByteArray = PackedByteArray()
var _world_size: Vector2 = Vector2.ONE
var _offset: Vector2 = Vector2.ZERO
var _zoom: float = 1.0
var _fit_enabled: bool = true
var _dragging: bool = false
var _crisp_lines: bool = false
var _drag_button: int = MOUSE_BUTTON_NONE
var _left_pan_enabled: bool = false
var _press_position: Vector2 = Vector2.ZERO
var _selection: Dictionary = {}
var _previous_size: Vector2 = Vector2.ZERO
var _touches: Dictionary[int, Vector2] = {}
var _touch_origin: Vector2 = Vector2.ZERO
var _touch_dragging: bool = false
var _touch_multiple: bool = false
var measure_submission: bool = false
var _draw_count: int = 0
var _draw_usec: int = 0
var _draw_max_usec: int = 0


func _ready() -> void:
	clip_contents = true
	mouse_default_cursor_shape = Control.CURSOR_DRAG
	if underwater_background:
		var water: ColorRect = ColorRect.new()
		water.name = "UnderwaterBackground"
		water.mouse_filter = Control.MOUSE_FILTER_IGNORE
		water.show_behind_parent = true
		water.material = UNDERWATER_MATERIAL
		add_child(water)
		water.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_previous_size = size
	resized.connect(_on_resized)


func set_snapshot(segments: PackedFloat64Array, colors: PackedInt32Array, width: float, height: float, alphas: PackedByteArray = PackedByteArray()) -> void:
	_segments = segments
	_colors = colors
	_alphas = alphas
	var dimensions: Vector2 = Vector2(width, height)
	if dimensions != _world_size:
		_world_size = dimensions
		_fit_enabled = true
	if _fit_enabled:
		fit_world()
	queue_redraw()


func set_network_presentation(commands: PackedFloat64Array) -> bool:
	if commands.size() % 6 != 0:
		return false
	for index: int in range(0, commands.size(), 6):
		for field: int in range(6):
			if not is_finite(commands[index + field]):
				return false
		if commands[index] not in [0.0, 1.0, 2.0]:
			return false
		var argb: float = commands[index + 5]
		if argb < 0.0 or argb > 4294967295.0 or argb != floor(argb):
			return false
	_network_presentation = commands
	queue_redraw()
	return true


func fit_world() -> void:
	_fit_enabled = true
	_zoom = minf(size.x / _world_size.x, size.y / _world_size.y) * 0.92
	_zoom = maxf(_zoom, MIN_ZOOM)
	_offset = (size - _world_size * _zoom) * 0.5
	view_changed.emit(_zoom)
	queue_redraw()


func zoom_at(factor: float, anchor: Vector2) -> void:
	if not is_finite(factor) or factor <= 0.0 or not is_finite(anchor.x) or not is_finite(anchor.y):
		return
	var next_zoom: float = clampf(_zoom * factor, MIN_ZOOM, MAX_ZOOM)
	var next_offset: Vector2 = anchor - (anchor - _offset) * (next_zoom / _zoom)
	if not _finite_vector(next_offset):
		return
	_fit_enabled = false
	_offset = next_offset
	_zoom = next_zoom
	view_changed.emit(_zoom)
	queue_redraw()


func set_camera_view(center: Vector2, zoom: float) -> void:
	if not _finite_vector(center) or not is_finite(zoom) or zoom < MIN_ZOOM or zoom > MAX_ZOOM:
		return
	if center.x < 0.0 or center.y < 0.0 or center.x > _world_size.x or center.y > _world_size.y:
		return
	var next_offset: Vector2 = size * 0.5 - center * zoom
	if not _finite_vector(next_offset):
		return
	_fit_enabled = false
	_zoom = zoom
	_offset = next_offset
	view_changed.emit(_zoom)
	queue_redraw()


func view_state() -> Dictionary:
	return {"center": (size * 0.5 - _offset) / _zoom, "zoom": _zoom, "world_size": _world_size}


func _finite_vector(value: Vector2) -> bool:
	return is_finite(value.x) and is_finite(value.y)


func _touch_pan(delta: Vector2) -> void:
	if delta == Vector2.ZERO or not _finite_vector(delta) or not _finite_vector(_offset + delta):
		return
	_fit_enabled = false
	_offset += delta
	manual_view_changed.emit()
	queue_redraw()


func _screen_touch(event: InputEventScreenTouch) -> void:
	if event.index < 0:
		return
	if event.canceled:
		if _touches.has(event.index):
			_touches.erase(event.index)
			_touch_multiple = true
			_touch_dragging = not _touches.is_empty()
		return
	if not _finite_vector(event.position):
		return
	if event.pressed:
		if _touches.has(event.index) or _touches.size() >= 2:
			return
		if _touches.is_empty():
			_touch_origin = event.position
			_touch_dragging = false
			_touch_multiple = false
		else:
			_touch_multiple = true
		_touches[event.index] = event.position
		grab_focus()
		manual_view_changed.emit()
	elif _touches.has(event.index):
		var tap: bool = _touches.size() == 1 and not _touch_dragging and not _touch_multiple
		_touches.erase(event.index)
		if tap and event.position.distance_squared_to(_touch_origin) < 16.0:
			organism_picked.emit((event.position - _offset) / _zoom, false, get_global_transform_with_canvas() * event.position)
		_touch_dragging = not _touches.is_empty()


func _screen_drag(event: InputEventScreenDrag) -> void:
	if not _touches.has(event.index) or not _finite_vector(event.position):
		return
	var previous: Vector2 = _touches[event.index]
	if _touches.size() == 1:
		_touches[event.index] = event.position
		if _touch_dragging:
			_touch_pan(event.position - previous)
		elif event.position.distance_squared_to(_touch_origin) >= 16.0:
			_touch_dragging = true
			_touch_pan(event.position - _touch_origin)
		return
	var points: Array[Vector2] = _touches.values()
	var old_center: Vector2 = points[0] * 0.5 + points[1] * 0.5
	var old_distance: float = points[0].distance_to(points[1])
	_touches[event.index] = event.position
	points = _touches.values()
	var new_center: Vector2 = points[0] * 0.5 + points[1] * 0.5
	var new_distance: float = points[0].distance_to(points[1])
	if old_distance > 0.0 and new_distance > 0.0 and is_finite(old_distance) and is_finite(new_distance):
		zoom_at(new_distance / old_distance, old_center)
		manual_view_changed.emit()
	_touch_pan(new_center - old_center)


func _on_resized() -> void:
	if _fit_enabled:
		fit_world()
	else:
		if not _selection.is_empty():
			var bounds: PackedInt32Array = _selection["bounds"]
			var center: Vector2 = _offset + Vector2(bounds[0] + bounds[2] * 0.5, bounds[1] + bounds[3] * 0.5) * _zoom
			if Rect2(Vector2.ZERO, _previous_size).has_point(center):
				var visible_center: Vector2 = Vector2(clampf(center.x, 8.0, maxf(8.0, size.x - 8.0)), clampf(center.y, 8.0, maxf(8.0, size.y - 8.0)))
				_offset += visible_center - center
		queue_redraw()
	_previous_size = size


func _gui_input(event: InputEvent) -> void:
	# Keep emulation enabled for Buttons, but handle each canvas pointer action only once
	if event.device == InputEvent.DEVICE_ID_EMULATION and (event is InputEventMouse or event is InputEventScreenTouch or event is InputEventScreenDrag):
		accept_event()
		return
	if event is InputEventScreenTouch:
		_screen_touch(event as InputEventScreenTouch)
		accept_event()
	elif event is InputEventScreenDrag:
		_screen_drag(event as InputEventScreenDrag)
		accept_event()
	elif event is InputEventKey:
		var key: InputEventKey = event as InputEventKey
		if not key.pressed or key.alt_pressed or key.ctrl_pressed or key.meta_pressed or key.shift_pressed:
			return
		var axis: int
		var direction: float
		match key.keycode:
			KEY_LEFT:
				axis = 0
				direction = -1.0
			KEY_RIGHT:
				axis = 0
				direction = 1.0
			KEY_UP:
				axis = 1
				direction = -1.0
			KEY_DOWN:
				axis = 1
				direction = 1.0
			_:
				return
		manual_view_changed.emit()
		if _fit_enabled:
			accept_event()
			return
		var extent: float = _world_size[axis] * _zoom
		var next_offset: float = (size[axis] - extent) * 0.5
		if extent > size[axis]:
			next_offset = clampf(_offset[axis] - direction * 10.0, size[axis] - extent, 0.0)
		if next_offset != _offset[axis]:
			_fit_enabled = false
			_offset[axis] = next_offset
			queue_redraw()
		accept_event()
	elif event is InputEventMouseButton:
		var button_event: InputEventMouseButton = event as InputEventMouseButton
		var button: int = button_event.button_index
		if button in [MOUSE_BUTTON_LEFT, MOUSE_BUTTON_MIDDLE, MOUSE_BUTTON_RIGHT]:
			if button_event.canceled:
				if button == _drag_button:
					_clear_mouse_drag()
			elif button_event.pressed:
				manual_view_changed.emit()
				grab_focus()
				_drag_button = button
				_left_pan_enabled = button == MOUSE_BUTTON_LEFT and not button_event.ctrl_pressed
				_press_position = button_event.position
				_dragging = false
			elif button == _drag_button:
				var click: bool = not _dragging and button != MOUSE_BUTTON_MIDDLE
				if _left_pan_enabled and button_event.position.distance_squared_to(_press_position) >= 16.0:
					click = false
				if click:
					var world_position: Vector2 = (button_event.position - _offset) / _zoom
					organism_picked.emit(world_position, button == MOUSE_BUTTON_RIGHT, button_event.global_position)
				_clear_mouse_drag()
			accept_event()
		elif button_event.pressed and button == MOUSE_BUTTON_WHEEL_UP:
			manual_view_changed.emit()
			zoom_at(1.2, button_event.position)
			accept_event()
		elif button_event.pressed and button == MOUSE_BUTTON_WHEEL_DOWN:
			manual_view_changed.emit()
			zoom_at(1.0 / 1.2, button_event.position)
			accept_event()
	elif event is InputEventMouseMotion and (_drag_button in [MOUSE_BUTTON_MIDDLE, MOUSE_BUTTON_RIGHT] or _left_pan_enabled):
		var motion: InputEventMouseMotion = event as InputEventMouseMotion
		if _left_pan_enabled and (motion.button_mask & MOUSE_BUTTON_MASK_LEFT) == 0:
			_clear_mouse_drag()
			accept_event()
			return
		if _dragging:
			_offset += motion.relative
		elif motion.position.distance_squared_to(_press_position) >= 16.0:
			_dragging = true
			_offset += motion.position - _press_position
		if _dragging:
			_fit_enabled = false
			manual_view_changed.emit()
			queue_redraw()
		accept_event()
	elif event is InputEventPanGesture:
		var pan: InputEventPanGesture = event as InputEventPanGesture
		_fit_enabled = false
		manual_view_changed.emit()
		_offset -= pan.delta * 12.0
		queue_redraw()
		accept_event()
	elif event is InputEventMagnifyGesture:
		var magnify: InputEventMagnifyGesture = event as InputEventMagnifyGesture
		manual_view_changed.emit()
		zoom_at(magnify.factor, magnify.position)
		accept_event()


func _clear_mouse_drag() -> void:
	_dragging = false
	_drag_button = MOUSE_BUTTON_NONE
	_left_pan_enabled = false


func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_WINDOW_FOCUS_OUT:
		_clear_mouse_drag()
		_touches.clear()
		_touch_dragging = false
		_touch_multiple = false


func _can_coalesce_points() -> bool:
	if self_modulate != Color.WHITE:
		return false
	var item: CanvasItem = self
	while item != null:
		if item.modulate != Color.WHITE or item.material != null or item.use_parent_material:
			return false
		if item.top_level:
			break
		item = item.get_parent() as CanvasItem
	return true


func set_crisp_lines(enabled: bool) -> void:
	if _crisp_lines != enabled:
		_crisp_lines = enabled
		queue_redraw()


func set_selection(data: Dictionary) -> void:
	_selection = data
	queue_redraw()


func follow_organism(center: PackedFloat64Array, max_velocity: float) -> void:
	if center.size() != 2:
		return
	_fit_enabled = false
	var span: Vector2 = size / _zoom
	var current: Vector2 = -_offset / _zoom
	var limit: int = _i32(2 * _java_int(max_velocity))
	for axis: int in range(2):
		var desired: int = _i32(int(center[axis]) - int(span[axis]) / 2)
		var lower: int = _i32(int(current[axis]) - limit)
		var upper: int = _i32(int(current[axis]) + limit)
		var value: int = maxi(mini(upper, desired), lower)
		if span[axis] < _world_size[axis]:
			_offset[axis] = -clampf(float(value), 0.0, _world_size[axis] - span[axis]) * _zoom
		else:
			_offset[axis] = (size[axis] - _world_size[axis] * _zoom) * 0.5
	queue_redraw()


func _java_int(value: float) -> int:
	if is_nan(value):
		return 0
	return int(clampf(value, -2147483648.0, 2147483647.0))


func _i32(value: int) -> int:
	return ((value + 2147483648) & 4294967295) - 2147483648


func _draw() -> void:
	var started: int = Time.get_ticks_usec() if measure_submission else 0
	if not underwater_background:
		draw_rect(Rect2(Vector2.ZERO, size), Color.BLACK)
	if _crisp_lines:
		_draw_crisp()
	else:
		_draw_smooth()
	_draw_selection()
	if measure_submission:
		var elapsed: int = Time.get_ticks_usec() - started
		_draw_count += 1
		_draw_usec += elapsed
		_draw_max_usec = maxi(_draw_max_usec, elapsed)


## CPU submission includes preparation and draw calls, not completed GPU work
func submission_metrics() -> Dictionary:
	return {"enabled": measure_submission, "draw_count": _draw_count, "total_usec": _draw_usec,
		"max_usec": _draw_max_usec, "segment_count": _colors.size()}


func _draw_selection() -> void:
	if _selection.is_empty():
		return
	var bounds: PackedInt32Array = _selection["bounds"]
	var width: int = _i32(bounds[2] - 1)
	var height: int = _i32(bounds[3] - 1)
	if width < 0 or height < 0:
		return
	var top_left: Vector2 = Vector2(bounds[0], bounds[1]) * _zoom + _offset
	var bottom_right: Vector2 = top_left + Vector2(width, height) * _zoom
	var color: Color = Color.WHITE if bool(_selection["infected"]) else Color("ffc800")
	if width == 0 or height == 0:
		draw_rect(Rect2(top_left - Vector2.ONE * 0.5, Vector2(width, height) * _zoom + Vector2.ONE), color)
		return
	draw_polyline(PackedVector2Array([top_left, Vector2(bottom_right.x, top_left.y), bottom_right, Vector2(top_left.x, bottom_right.y), top_left]), color, 1.0, false)


func _draw_network_presentation() -> void:
	for index: int in range(0, _network_presentation.size(), 6):
		var kind: int = int(_network_presentation[index])
		var start: Vector2 = Vector2(_network_presentation[index + 1], _network_presentation[index + 2]) * _zoom + _offset
		var dimensions: Vector2 = Vector2(_network_presentation[index + 3], _network_presentation[index + 4])
		var argb: int = int(_network_presentation[index + 5])
		var color: Color = Color(float((argb >> 16) & 255) / 255.0, float((argb >> 8) & 255) / 255.0, float(argb & 255) / 255.0, float((argb >> 24) & 255) / 255.0)
		if kind == 2:
			var end: Vector2 = dimensions * _zoom + _offset
			if start == end:
				draw_rect(Rect2(start - Vector2.ONE * 0.5, Vector2.ONE), color)
			else:
				draw_line(start, end, color, 1.0, not _crisp_lines)
		elif kind == 1:
			if dimensions.x > 0.0 and dimensions.y > 0.0:
				draw_rect(Rect2(start, dimensions * _zoom), color)
		elif dimensions.x >= 0.0 and dimensions.y >= 0.0:
			var end: Vector2 = start + dimensions * _zoom
			if dimensions.x == 0.0 and dimensions.y == 0.0:
				draw_rect(Rect2(start - Vector2.ONE * 0.5, Vector2.ONE), color)
			elif dimensions.x == 0.0 or dimensions.y == 0.0:
				draw_line(start, end, color, 1.0, not _crisp_lines)
			else:
				draw_polyline(PackedVector2Array([start, Vector2(end.x, start.y), end, Vector2(start.x, end.y), start]), color, 1.0, not _crisp_lines)


func _draw_smooth() -> void:
	_draw_network_presentation()
	var can_coalesce: bool = _can_coalesce_points()
	var prepared: Dictionary
	if can_coalesce and clip_contents and get_global_transform_with_canvas() == Transform2D.IDENTITY \
		and get_viewport_transform() == Transform2D.IDENTITY and get_viewport_rect() == Rect2(Vector2.ZERO, size):
		if RenderingServer.get_current_rendering_method() == "gl_compatibility":
			prepared = BgrSimulation.prepare_smooth_aa_viewport_batch(_segments, _colors, _alphas, _zoom, _offset, can_coalesce, size)
		else:
			prepared = BgrSimulation.prepare_smooth_viewport_batch(_segments, _colors, _alphas, _zoom, _offset, can_coalesce, size)
	else:
		prepared = BgrSimulation.prepare_smooth_batch(_segments, _colors, _alphas, _zoom, _offset, can_coalesce)
	if not bool(prepared["eligible"]):
		_draw_smooth_fallback()
		return
	var commands: PackedInt32Array = prepared["commands"]
	if bool(prepared.get("aa", false)):
		var vertices: Array[PackedVector2Array] = prepared["aa_points"]
		var shades: Array[PackedColorArray] = prepared["aa_colors"]
		var indices: Array[PackedInt32Array] = prepared["aa_indices"]
		var rects: Array[Rect2] = prepared["point_rects"]
		var rect_colors: PackedColorArray = prepared["point_colors"]
		for command: int in commands:
			if command >= 0:
				RenderingServer.canvas_item_add_triangle_array(get_canvas_item(), indices[command], vertices[command], shades[command])
			else:
				var index: int = -command - 1
				draw_rect(rects[index], rect_colors[index])
		return
	var line_points: Array[PackedVector2Array] = prepared["line_points"]
	var line_colors: Array[PackedColorArray] = prepared["line_colors"]
	var point_rects: Array[Rect2] = prepared["point_rects"]
	var point_colors: PackedColorArray = prepared["point_colors"]
	for command: int in commands:
		if command >= 0:
			draw_multiline_colors(line_points[command], line_colors[command], 1.0, true)
		else:
			var index: int = -command - 1
			draw_rect(point_rects[index], point_colors[index])


func _draw_smooth_fallback() -> void:
	var batch_points: PackedVector2Array = PackedVector2Array()
	var batch_colors: PackedColorArray = PackedColorArray()
	var pending_point: Vector2 = Vector2.ZERO
	var pending_color: Color = Color.WHITE
	var has_point: bool = false
	var can_coalesce: bool = _can_coalesce_points()
	for index: int in range(_colors.size()):
		var start: int = index * 4
		var from_point: Vector2 = Vector2(_segments[start] * _zoom + _offset.x, _segments[start + 1] * _zoom + _offset.y)
		var to_point: Vector2 = Vector2(_segments[start + 2] * _zoom + _offset.x, _segments[start + 3] * _zoom + _offset.y)
		var rgb: int = _colors[index]
		var color: Color = Color(float((rgb >> 16) & 255) / 255.0, float((rgb >> 8) & 255) / 255.0, float(rgb & 255) / 255.0)
		if not _alphas.is_empty():
			color.a = float(_alphas[index]) / 255.0
		if from_point == to_point:
			if not batch_colors.is_empty():
				draw_multiline_colors(batch_points, batch_colors, 1.0, true)
				batch_points.clear()
				batch_colors.clear()
			if can_coalesce and color.a == 1.0:
				if has_point and pending_point != from_point:
					draw_rect(Rect2(pending_point - Vector2.ONE * 0.5, Vector2.ONE), pending_color)
				pending_point = from_point
				pending_color = color
				has_point = true
			else:
				if has_point:
					draw_rect(Rect2(pending_point - Vector2.ONE * 0.5, Vector2.ONE), pending_color)
					has_point = false
				draw_rect(Rect2(from_point - Vector2.ONE * 0.5, Vector2.ONE), color)
		else:
			if has_point:
				draw_rect(Rect2(pending_point - Vector2.ONE * 0.5, Vector2.ONE), pending_color)
				has_point = false
			batch_points.append(from_point)
			batch_points.append(to_point)
			batch_colors.append(color)
	if has_point:
		draw_rect(Rect2(pending_point - Vector2.ONE * 0.5, Vector2.ONE), pending_color)
	if not batch_colors.is_empty():
		draw_multiline_colors(batch_points, batch_colors, 1.0, true)


func _prepare_crisp_batch() -> Dictionary:
	if not _can_coalesce_points():
		return {"ok": true, "eligible": false}
	return BgrSimulation.prepare_crisp_batch(_segments, _colors, _alphas, _zoom, _offset)


func _draw_crisp() -> void:
	_draw_network_presentation()
	var prepared: Dictionary = _prepare_crisp_batch()
	if not bool(prepared["ok"]):
		push_error(str(prepared["error"]))
		return
	if bool(prepared["eligible"]):
		var points: PackedVector2Array = prepared["points"]
		var colors: PackedColorArray = prepared["colors"]
		if not colors.is_empty():
			draw_multiline_colors(points, colors, 1.0, false)
		return
	var batch_points: PackedVector2Array = PackedVector2Array()
	var batch_colors: PackedColorArray = PackedColorArray()
	var can_combine: bool = _can_coalesce_points()
	for index: int in range(_colors.size()):
		var start: int = index * 4
		var from_point: Vector2 = Vector2(_segments[start] * _zoom + _offset.x, _segments[start + 1] * _zoom + _offset.y)
		var to_point: Vector2 = Vector2(_segments[start + 2] * _zoom + _offset.x, _segments[start + 3] * _zoom + _offset.y)
		var rgb: int = _colors[index]
		var color: Color = Color(float((rgb >> 16) & 255) / 255.0, float((rgb >> 8) & 255) / 255.0, float(rgb & 255) / 255.0)
		if not _alphas.is_empty():
			color.a = float(_alphas[index]) / 255.0
		if can_combine and color.a == 1.0:
			if from_point == to_point:
				batch_points.append(from_point - Vector2(0.5, 0.0))
				batch_points.append(from_point + Vector2(0.5, 0.0))
			else:
				batch_points.append(from_point)
				batch_points.append(to_point)
			batch_colors.append(color)
		else:
			if not batch_colors.is_empty():
				draw_multiline_colors(batch_points, batch_colors, 1.0, false)
				batch_points.clear()
				batch_colors.clear()
			if from_point == to_point:
				draw_rect(Rect2(from_point - Vector2.ONE * 0.5, Vector2.ONE), color)
			else:
				draw_line(from_point, to_point, color, 1.0, false)
	if not batch_colors.is_empty():
		draw_multiline_colors(batch_points, batch_colors, 1.0, false)


func native_fit_enabled() -> bool:
	return _fit_enabled


func native_view_offset() -> Vector2:
	return _offset
