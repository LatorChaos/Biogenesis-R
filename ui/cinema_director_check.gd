extends SceneTree

const Director = preload("res://ui/cinema_director.gd")
var _checks: int = 0
var _failures: Array[String] = []
var _world: Vector2 = Vector2(7000, 7000)
var _viewport: Vector2 = Vector2(1280, 800)
var _view: Dictionary = {}


func _check(condition: bool, message: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(message)


func _feeding(energy: float = 5.0, live: bool = true, batch: int = 1) -> Dictionary:
	return {"available": true, "epoch": 1, "scope": "latest_frontend_advance_batch", "energy_unit": "gross_source_energy_before_costs", "event": {
		"batch": batch, "tick_at_transfer": batch, "gross_energy": energy,
		"predator_alive_at_transfer": true, "prey_alive_at_transfer": live,
		"predator": {"id": 10, "bounds": PackedInt32Array([100, 200, 20, 30])},
		"prey": {"id": 20, "bounds": PackedInt32Array([120, 220, 10, 20])}}}


func _advance(director: Director, delta: float, world: Vector2, viewport: Vector2, token: int = 0) -> Dictionary:
	var target: Dictionary = director.advance(delta, world, viewport, token, _view)
	if not target.is_empty():
		_view = target
	return target


func _initialize() -> void:
	var director: Director = Director.new()
	director.hold_seconds = 5.0
	_check(_advance(director, 0.1, _world, _viewport)["kind"] == "overview", "Start with a broad view")
	for index: int in range(801):
		_advance(director, 0.25, _world, _viewport)
	_check(_advance(director, 0.1, _world, _viewport)["kind"] == "scenic", "Mix scenic views without requiring organisms")
	director.set_scene({"ok": true, "world_ready": true, "presentation_epoch": 1,
		"subjects": [{"token": 11, "id": 1, "alive": true, "center": PackedFloat64Array([300.0, 400.0]), "bounds": PackedInt32Array([290, 390, 20, 20])}],
		"records": [{"present": true, "subject_token": 11}]})
	var featured: Dictionary = _advance(director, 0.1, _world, _viewport, 11)
	_check(featured["kind"] == "living" and featured["focus"] == Vector2(300, 400), "Frame the actual living subject featured by a record card")
	for index: int in range(1601):
		_advance(director, 0.25, _world, _viewport)
	var portrait: Dictionary = _advance(director, 0.1, _world, _viewport)
	_check(portrait["kind"] == "living" and portrait["focus"] == Vector2(300, 400), "Frame an actual living subject")
	_check(director.follow_token == 11, "Retain the stable follow token")
	director.set_scene({"ok": true, "world_ready": true, "presentation_epoch": 1, "subjects": [], "records": []})
	_check(_advance(director, 0.1, _world, _viewport, 11)["kind"] != "living", "Reject a featured token that is no longer alive")
	_check(director.follow_token == 0 and _advance(director, 0.1, _world, _viewport)["kind"] != "living", "Stop following a dead or removed subject")
	director.clear()
	director.observe_feeding(_feeding(0.0), Rect2(Vector2.ZERO, _world))
	_check(_advance(director, 0.1, _world, _viewport)["kind"] != "feeding", "Zero transfer is not predation")
	director.clear()
	director.observe_feeding(_feeding(5.0, false), Rect2(Vector2.ZERO, _world))
	_check(_advance(director, 0.1, _world, _viewport)["kind"] != "feeding", "Dead-peer scavenging is not a live action shot")
	director.clear()
	var feeding: Dictionary = _feeding()
	director.observe_feeding(feeding, Rect2(Vector2.ZERO, _world))
	var action: Dictionary = _advance(director, 0.1, _world, _viewport)
	_check(action["kind"] == "feeding" and action["focus"] == Vector2(115, 220), "Positive live transfer frames both stamped actors")
	_check(action["zoom"] <= 800.0 / 560.0, "Keep context around action close-ups in a large world")
	var start: Dictionary = {"center": _world * 0.5, "zoom": 0.1}
	var blended: Dictionary = Director.blend_view(start, action, _world, _viewport, 0.25)
	var span: Vector2 = _viewport / float(blended["zoom"])
	_check(Rect2((blended["center"] as Vector2) - span * 0.5, span).has_point(action["focus"]), "Keep an already visible transfer in frame while zooming")
	var outside: Dictionary = Director.blend_view({"center": Vector2(-1000, 9000), "zoom": 0.1}, action, _world, _viewport, 0.016)
	_check(outside["center"].x >= 0.0 and outside["center"].x <= 7000.0 and outside["center"].y >= 0.0 and outside["center"].y <= 7000.0, "Recover automatic viewing after a pan outside the world")
	_check(feeding == _feeding(), "Presentation leaves its packet unchanged")
	for index: int in range(33):
		director.observe_feeding(_feeding(5.0, true, index + 2), Rect2(Vector2.ZERO, _world))
		_advance(director, 0.25, _world, _viewport)
	_check(_advance(director, 0.1, _world, _viewport)["kind"] != "feeding", "Continuous feeding cannot hold the action camera indefinitely")
	director.cancel_action()
	_check(_advance(director, 0.1, _world, _viewport)["kind"] != "feeding", "Manual intervention cancels an action shot")
	director.clear()
	_check(director.follow_token == 0, "World replacement clears the follow token")
	director.observe_feeding(_feeding(), Rect2(3000, 3000, 200, 200))
	_check(_advance(director, 0.1, _world, _viewport)["kind"] != "feeding", "Ignore distant transfers instead of panning across an empty world")
	director.clear()
	director.observe_feeding(_feeding(), Rect2(Vector2.ZERO, _world))
	_check(_advance(director, 3.5, _world, _viewport)["kind"] != "feeding", "A stalled frame cannot revive an old transfer")
	_check(_advance(director, NAN, _world, _viewport).is_empty(), "Reject invalid timing")
	_check(_advance(director, 0.1, Vector2.ZERO, _viewport).is_empty(), "Reject unavailable world geometry")
	print("BGR_CINEMA " + JSON.stringify({"checks": _checks, "failures": _failures, "result": "PASS" if _failures.is_empty() else "FAIL"}))
	quit(0 if _failures.is_empty() else 1)
