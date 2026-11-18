extends RefCounted

var settled_hold_seconds: float = 200.0
var motion_rate_scale: float = 0.125

var hold_seconds: float = 20.0
var follow_token: int = 0
var _clock: float = 0.0
var _epoch: int = -1
var _subjects: Dictionary = {}
var _records: Array = []
var _action: Rect2
var _action_left: float = 0.0
var _action_age: float = 0.0
var _action_cooldown: float = 0.0
var _predator_id: int = -1
var _last_event: Vector3i = Vector3i(-1, -1, -1)
var _portrait_round: int = -1
var _shot: int = 0
var _settled_seconds: float = 0.0
var _scenic_center: Vector2
var _scenic_ready: bool = false
var _shot_frame: Dictionary = {}
var _shot_token: int = 0
var _settled: bool = false


func set_scene(packet: Dictionary) -> void:
	if not packet.get("ok", false) or not packet.get("world_ready", false):
		clear()
		return
	var epoch: int = int(packet.get("presentation_epoch", -1))
	if epoch != _epoch:
		clear()
		_epoch = epoch
	_subjects.clear()
	for subject: Dictionary in packet.get("subjects", []):
		var center: Variant = subject.get("center")
		var bounds: Variant = subject.get("bounds")
		if subject.get("alive", false) and int(subject.get("token", 0)) > 0 and center is PackedFloat64Array and center.size() == 2 and bounds is PackedInt32Array and bounds.size() == 4 and is_finite(center[0]) and is_finite(center[1]) and bounds[2] >= 0 and bounds[3] >= 0:
			_subjects[int(subject["token"])] = subject
	_records = packet.get("records", [])
	if not _subjects.has(follow_token):
		follow_token = 0
	if _shot_token > 0 and not _subjects.has(_shot_token):
		_release_shot(false)


func clear() -> void:
	_clock = 0.0
	_action_cooldown = 0.0
	_subjects.clear()
	_records.clear()
	follow_token = 0
	_epoch = -1
	_portrait_round = -1
	_shot = 0
	_settled_seconds = 0.0
	_scenic_ready = false
	_release_shot(false)
	cancel_action()
	_last_event = Vector3i(-1, -1, -1)


func cancel_action() -> void:
	_action_left = 0.0
	_action_age = 0.0
	_predator_id = -1


## Frame the actual transfer location, never infer a hunt from lifetime counters
func observe_feeding(packet: Dictionary, visible_world: Rect2) -> void:
	if not packet.get("available", false) or packet.get("scope") != "latest_frontend_advance_batch" or packet.get("energy_unit") != "gross_source_energy_before_costs":
		return
	var event: Dictionary = packet.get("event", {})
	if not event.get("predator_alive_at_transfer", false) or not event.get("prey_alive_at_transfer", false):
		return
	var energy: float = float(event.get("gross_energy", 0.0))
	if not is_finite(energy) or energy <= 0.0:
		return
	var identity: Vector3i = Vector3i(int(packet.get("epoch", -1)), int(event.get("batch", -1)), int(event.get("tick_at_transfer", -1)))
	if identity == _last_event:
		return
	_last_event = identity
	var predator: Dictionary = event.get("predator", {})
	var prey: Dictionary = event.get("prey", {})
	var first: Variant = predator.get("bounds")
	var second: Variant = prey.get("bounds")
	if not first is PackedInt32Array or first.size() != 4 or not second is PackedInt32Array or second.size() != 4 or first[2] < 0 or first[3] < 0 or second[2] < 0 or second[3] < 0:
		return
	var area: Rect2 = Rect2(first[0], first[1], first[2], first[3]).merge(Rect2(second[0], second[1], second[2], second[3]))
	if not visible_world.has_area() or not visible_world.has_point(area.get_center()):
		return
	var same_subject: bool = int(predator.get("id", -1)) == _predator_id
	if _action_left > 0.0:
		if not same_subject or _action_age >= 8.0:
			return
	elif _action_cooldown > 0.0:
		return
	else:
		_action_age = 0.0
	_predator_id = int(predator.get("id", -1))
	_action = area
	_action_left = 3.0


## Return presentation coordinates only; no simulation or random calls
func advance(delta: float, world: Vector2, viewport: Vector2, featured_token: int = 0, view: Dictionary = {}) -> Dictionary:
	if not is_finite(delta) or delta <= 0.0 or not world.is_finite() or not viewport.is_finite() or world.x <= 0.0 or world.y <= 0.0 or viewport.x <= 0.0 or viewport.y <= 0.0:
		return {}
	_clock += delta
	_action_cooldown = maxf(0.0, _action_cooldown - delta)
	if _action_left > 0.0:
		_action_left = maxf(0.0, _action_left - delta)
		_action_age += delta
		if _action_left <= 0.0 or _action_age >= 8.0:
			cancel_action()
	if not _shot_frame.is_empty():
		# A moving subject must not move the destination used to detect arrival
		var entry: Dictionary = _frame(_shot_frame["focus"], float(_shot_frame["height"]), world, viewport, _shot_frame["kind"])
		if _shot_token > 0 and not _settled:
			var center: PackedFloat64Array = _subjects[_shot_token]["center"]
			var subject: Vector2 = Vector2(center[0], center[1])
			var span: Vector2 = viewport / float(entry["zoom"])
			var visible: Rect2 = Rect2((entry["center"] as Vector2) - span * 0.5, span).grow(0.002)
			if visible.has_point(subject):
				entry["focus"] = subject
			else:
				if featured_token == _shot_token:
					featured_token = 0
				follow_token = 0
				_release_shot(true)
		if not _shot_frame.is_empty():
			if not _settled and not view.is_empty() and _arrived(view, entry, viewport):
				_settled = true
			if _settled:
				_settled_seconds += delta
			if _settled_seconds < settled_hold_seconds:
				return _living_frame(_shot_token, world, viewport) if _settled and _shot_token > 0 else entry
			_release_shot(true)
	if featured_token > 0 and _subjects.has(featured_token):
		cancel_action()
		return _begin_shot(_living_frame(featured_token, world, viewport), featured_token, viewport)
	if _action_left > 0.0:
		if _action_cooldown <= 0.0:
			_action_cooldown = maxf(10.0, hold_seconds)
		var context: float = clampf(minf(world.x, world.y) * 0.08, 140.0, 560.0)
		return _frame(_action.get_center(), maxf(context, maxf(_action.size.y * 3.0, _action.size.x * viewport.y / viewport.x * 3.0)), world, viewport, "feeding")
	var target: Dictionary = _atmospheric_frame(world, viewport)
	return _begin_shot(target, follow_token if target["kind"] == "living" else 0, viewport)


func _begin_shot(frame: Dictionary, token: int, viewport: Vector2) -> Dictionary:
	_shot_frame = frame.duplicate()
	_shot_frame["height"] = viewport.y / float(frame["zoom"])
	_shot_token = token
	_settled = false
	_settled_seconds = 0.0
	return frame


func _release_shot(next: bool) -> void:
	_shot_frame.clear()
	_shot_token = 0
	_settled = false
	_settled_seconds = 0.0
	if next:
		_shot += 1
		_scenic_ready = false


func _atmospheric_frame(world: Vector2, viewport: Vector2) -> Dictionary:
	var mode: int = _shot % 3
	if mode == 2 and not _subjects.is_empty():
		var round: int = _shot / 3
		if round != _portrait_round or not _subjects.has(follow_token):
			_portrait_round = round
			var eligible: Array[int] = []
			for record: Dictionary in _records:
				var token: int = int(record.get("subject_token", 0))
				if record.get("present", false) and _subjects.has(token) and not eligible.has(token):
					eligible.append(token)
			if eligible.is_empty():
				eligible.assign(_subjects.keys())
			follow_token = eligible[round % eligible.size()]
		return _living_frame(follow_token, world, viewport)
	follow_token = 0
	if mode == 0:
		return _frame(world * 0.5, maxf(world.y, world.x * viewport.y / viewport.x), world, viewport, "overview")
	if not _scenic_ready:
		_scenic_center = world * 0.5 + Vector2(sin(_clock / 55.0), cos(_clock / 71.0)) * world * 0.18
		_scenic_ready = true
	return _frame(_scenic_center, minf(world.x, world.y) * 0.35, world, viewport, "scenic")


static func _arrived(view: Dictionary, target: Dictionary, viewport: Vector2) -> bool:
	var zoom: float = float(view["zoom"])
	return (view["center"] as Vector2).distance_to(target["center"]) * zoom <= 1.0 \
		and absf(viewport.y / zoom - viewport.y / float(target["zoom"])) <= viewport.y / float(target["zoom"]) * 0.001


func _living_frame(token: int, world: Vector2, viewport: Vector2) -> Dictionary:
	follow_token = token
	var actor: Dictionary = _subjects[token]
	var center: PackedFloat64Array = actor["center"]
	var bounds: PackedInt32Array = actor["bounds"]
	var context: float = clampf(minf(world.x, world.y) * 0.1, 220.0, 700.0)
	return _frame(Vector2(center[0], center[1]), maxf(context, maxf(bounds[3] * 5.0, bounds[2] * viewport.y / viewport.x * 5.0)), world, viewport, "living")


func _frame(center: Vector2, height: float, world: Vector2, viewport: Vector2, kind: String) -> Dictionary:
	var frame: Dictionary = _inside_world(center, viewport.y / maxf(1.0, height), world, viewport)
	frame["kind"] = kind
	frame["focus"] = center.clamp(Vector2.ZERO, world)
	return frame


static func _inside_world(center: Vector2, zoom: float, world: Vector2, viewport: Vector2, focus: Variant = null) -> Dictionary:
	zoom = maxf(zoom, maxf(viewport.x / world.x, viewport.y / world.y))
	var half: Vector2 = (viewport / (2.0 * zoom)).min(world * 0.5)
	center = center.clamp(half, world - half)
	if focus is Vector2:
		center = center.clamp((focus - half).max(half), (focus + half).min(world - half))
	return {"center": center, "zoom": zoom}


## Blend visible extents while keeping automatic views inside the world
static func blend_view(view: Dictionary, target: Dictionary, world: Vector2, viewport: Vector2, delta: float, motion_scale: float = 0.125) -> Dictionary:
	if not is_finite(delta) or delta <= 0.0 or not world.is_finite() or not viewport.is_finite() or world.x <= 0.0 or world.y <= 0.0 or viewport.x <= 0.0 or viewport.y <= 0.0:
		return {}
	var focus: Vector2 = target.get("focus", target["center"])
	var original_half: Vector2 = viewport / (2.0 * float(view["zoom"]))
	var distance: Vector2 = (focus - (view["center"] as Vector2)).abs()
	var keep_focus: bool = target["kind"] in ["feeding", "living"] and distance.x <= original_half.x + 0.002 and distance.y <= original_half.y + 0.002
	var start: Dictionary = _inside_world(view["center"], float(view["zoom"]), world, viewport, focus if keep_focus else null)
	var finish: Dictionary = _inside_world(target["center"], float(target["zoom"]), world, viewport)
	var weight: float = 1.0 - exp(-minf(delta, 0.25) * motion_scale * (1.8 if target["kind"] == "feeding" else 0.5))
	var center: Vector2 = (start["center"] as Vector2).lerp(finish["center"], weight)
	var height: float = lerpf(viewport.y / float(start["zoom"]), viewport.y / float(finish["zoom"]), weight)
	return _inside_world(center, viewport.y / maxf(1.0, height), world, viewport, focus if keep_focus else null)
