extends SceneTree

const Director = preload("res://ui/cinema_director.gd")
var _checks: int = 0
var _failures: Array[String] = []


func _check(condition: bool, message: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(message)


func _inside(frame: Dictionary, world: Vector2, viewport: Vector2) -> bool:
	var span: Vector2 = viewport / float(frame["zoom"])
	var first: Vector2 = frame["center"] - span * 0.5
	var last: Vector2 = first + span
	return first.x >= -0.002 and first.y >= -0.002 and last.x <= world.x + 0.002 and last.y <= world.y + 0.002


func _contains(frame: Dictionary, point: Vector2, viewport: Vector2) -> bool:
	var half: Vector2 = viewport / (2.0 * float(frame["zoom"]))
	var distance: Vector2 = (point - frame["center"]).abs()
	return distance.x <= half.x + 0.002 and distance.y <= half.y + 0.002


func _scene(center: Vector2) -> Dictionary:
	return {"ok": true, "world_ready": true, "presentation_epoch": 1,
		"subjects": [{"token": 11, "alive": true, "center": PackedFloat64Array([center.x, center.y]),
			"bounds": PackedInt32Array([int(center.x), int(center.y), 20, 20])}],
		"records": [{"present": true, "subject_token": 11}]}


func _initialize() -> void:
	for world: Vector2 in [Vector2(7000, 7000), Vector2(8000, 2000), Vector2(2000, 8000)]:
		for viewport: Vector2 in [Vector2(1280, 800), Vector2(800, 1280), Vector2(1920, 1080)]:
			var director: Director = Director.new()
			for shot: int in range(3):
				var target: Dictionary = director.advance(0.1, world, viewport)
				_check(_inside(target, world, viewport), "Atmospheric target exposes outside the world")
				director.advance(200.1, world, viewport, 0, target)
			for point: Vector2 in [Vector2.ZERO, Vector2(world.x, 0), world, Vector2(0, world.y), world * 0.5]:
				director.clear()
				director.set_scene(_scene(point))
				var target: Dictionary = director.advance(0.1, world, viewport, 11)
				_check(_inside(target, world, viewport) and _contains(target, point, viewport), "Edge subject target loses world coverage or subject")
				var view: Dictionary = {"center": world * 0.5, "zoom": minf(viewport.x / world.x, viewport.y / world.y) * 0.92}
				var transition_inside: bool = true
				var subject_visible: bool = true
				for frame: int in range(180):
					view = Director.blend_view(view, target, world, viewport, 1.0 / 30.0)
					transition_inside = transition_inside and _inside(view, world, viewport)
					subject_visible = subject_visible and _contains(view, point, viewport)
				_check(transition_inside, "Interpolated view exposes outside the world")
				_check(subject_visible, "Transition loses an already visible edge subject: " + str(world) + "/" + str(viewport) + "/" + str(point) + "/" + str(view))
				view = Director.blend_view({"center": Vector2(-1000, world.y + 2000), "zoom": 0.01}, target, world, viewport, 0.25)
				_check(_inside(view, world, viewport), "Automatic re-entry from manual pan exposes outside the world")
	var director: Director = Director.new()
	var world: Vector2 = Vector2(7000, 7000)
	var viewport: Vector2 = Vector2(1280, 800)
	var target: Dictionary = director.advance(0.1, world, viewport)
	var distant: Dictionary = {"center": Vector2(500, 500), "zoom": 1.0}
	_check(director.advance(500.0, world, viewport, 0, distant)["kind"] == "overview", "Travel consumed the settled hold")
	_check(director.advance(199.9, world, viewport, 0, target)["kind"] == "overview", "Settled hold ended before ten original holds")
	_check(director.advance(0.2, world, viewport, 0, target)["kind"] == "scenic", "Settled hold does not finish after 200 seconds")
	target = director.advance(0.1, world, viewport)
	_check(director.advance(500.0, world, viewport, 0, distant) == target, "Scenic target drifts during travel")
	_check(director.advance(199.9, world, viewport, 0, target)["kind"] == "scenic", "Scenic travel consumed the settled hold")
	director.set_scene(_scene(Vector2(3500, 3500)))
	target = director.advance(0.1, world, viewport)
	director.advance(200.1, world, viewport, 0, target)
	target = director.advance(0.1, world, viewport)
	_check(director.advance(200.1, world, viewport, 0, target)["kind"] == "living", "Living portrait priority is absent after atmospheric holds")
	director.clear()
	director.settled_hold_seconds = 10.0
	target = director.advance(0.1, world, viewport)
	_check(director.advance(10.1, world, viewport, 0, target)["kind"] == "scenic", "Configured settled hold is ignored")
	for geometry: Vector2 in [Vector2(NAN, 1), Vector2(1, INF)]:
		_check(director.advance(0.1, geometry, viewport).is_empty(), "Invalid world geometry was accepted")
		_check(director.advance(0.1, world, geometry).is_empty(), "Invalid viewport geometry was accepted")
	print("BGR_CINEMA_FRAMING " + JSON.stringify({"checks": _checks, "failures": _failures, "result": "PASS" if _failures.is_empty() else "FAIL"}))
	quit(0 if _failures.is_empty() else 1)
