extends SceneTree

const Director = preload("res://ui/cinema_director.gd")
var _checks: int = 0
var _failures: Array[String] = []
var _world: Vector2 = Vector2(7000, 7000)
var _viewport: Vector2 = Vector2(1280, 800)


func _check(condition: bool, message: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(message)


func _scene(tokens: Array[int], epoch: int = 1) -> Dictionary:
	var subjects: Array = []
	for token: int in tokens:
		subjects.append({"token": token, "alive": true, "center": PackedFloat64Array([3000.0 + token, 3500.0]), "bounds": PackedInt32Array([3000 + token, 3500, 20, 20])})
	return {"ok": true, "world_ready": true, "presentation_epoch": epoch, "subjects": subjects, "records": []}


func _feeding(batch: int, predator_id: int = 10) -> Dictionary:
	return {"available":true,"epoch":1,"scope":"latest_frontend_advance_batch","energy_unit":"gross_source_energy_before_costs","event":{
		"batch":batch,"tick_at_transfer":batch,"gross_energy":5.0,"predator_alive_at_transfer":true,"prey_alive_at_transfer":true,
		"predator":{"id":predator_id,"bounds":PackedInt32Array([3400,3500,20,30])},"prey":{"id":20,"bounds":PackedInt32Array([3420,3520,10,20])}}}


func _initialize() -> void:
	var director: Director = Director.new()
	director.set_scene(_scene([11, 22]))
	var target: Dictionary = director.advance(0.1, _world, _viewport)
	var traveling: Dictionary = {"center":Vector2(1000,1000),"zoom":1.0}
	_check(director.advance(60.0, _world, _viewport, 11, traveling) == target, "A card retargets the traveling atmospheric shot")
	var stayed: bool = true
	for second: int in range(199):
		director.observe_feeding(_feeding(second + 1), Rect2(Vector2.ZERO, _world))
		var held: Dictionary = director.advance(1.0, _world, _viewport, 11 if second % 2 == 0 else 22, target)
		stayed = stayed and held == target
	_check(stayed, "Cards or feeding interrupt the 200-second settled hold")
	var featured: Dictionary = director.advance(1.0, _world, _viewport, 22, target)
	_check(featured["kind"] == "living" and featured["focus"] == Vector2(3022,3500) and director.follow_token == 22, "A valid champion is not selected between holds")
	var living_stayed: bool = true
	for second: int in range(199):
		director.observe_feeding(_feeding(1000 + second), Rect2(Vector2.ZERO, _world))
		var held: Dictionary = director.advance(1.0, _world, _viewport, 11, featured)
		living_stayed = living_stayed and held["kind"] == "living" and held["focus"] == featured["focus"] and director.follow_token == 22
	_check(living_stayed, "A later card or feeding replaces the held living subject")
	director.set_scene(_scene([11]))
	var replacement: Dictionary = director.advance(0.1, _world, _viewport, 11, featured)
	_check(replacement["focus"] == Vector2(3011,3500) and director.follow_token == 11, "A removed subject remains held")
	director.clear()
	director.observe_feeding(_feeding(2000), Rect2(Vector2.ZERO, _world))
	_check(director.advance(0.1, _world, _viewport)["kind"] == "feeding", "Manual clear does not release the hold for a fresh action")
	_check(director.advance(3.1, _world, _viewport)["kind"] != "feeding", "A stale feeding shot acquires the long hold")
	director.clear()
	director.set_scene(_scene([11]))
	target = director.advance(0.1, _world, _viewport)
	director.advance(199.0, _world, _viewport, 0, target)
	director.observe_feeding(_feeding(3000), Rect2(Vector2.ZERO, _world))
	_check(director.advance(1.0, _world, _viewport, 0, target)["kind"] == "feeding", "A fresh real action is unavailable between settled holds")
	_check(director.advance(3.1, _world, _viewport)["kind"] != "feeding", "Expired between-hold feeding remains labeled active")
	director.clear()
	director.set_scene(_scene([11]))
	target = director.advance(0.1, _world, _viewport)
	director.advance(195.0, _world, _viewport, 0, target)
	var visible_world: Rect2 = Rect2(Vector2.ZERO, _world)
	director.observe_feeding(_feeding(3001), visible_world)
	_check(director.advance(3.1, _world, _viewport, 0, target) == target, "An early feeding event interrupts the settled hold")
	director.observe_feeding(_feeding(3002, 12), visible_world)
	_check(director.advance(2.0, _world, _viewport, 0, target)["kind"] == "feeding", "Unshown feeding starts cooldown and blocks fresh feeding at the cut")
	director.set_scene(_scene([22], 2))
	var new_world: Dictionary = director.advance(0.1, _world, _viewport, 22)
	_check(new_world["focus"] == Vector2(3022,3500) and director.follow_token == 22, "World replacement retains the old shot gate")
	director.set_scene({"ok":false,"world_ready":false})
	_check(director.advance(0.1, _world, _viewport, 22)["kind"] != "living", "Unavailable world data retains a living shot")
	_check_moving_subject()
	print("BGR_CINEMA_HOLD_PRIORITY " + JSON.stringify({"checks":_checks,"failures":_failures,"result":"PASS" if _failures.is_empty() else "FAIL"}))
	quit(0 if _failures.is_empty() else 1)


func _check_moving_subject() -> void:
	var director: Director = Director.new()
	director.settled_hold_seconds = 5.0
	var scene: Dictionary = _scene([11])
	director.set_scene(scene)
	var entry: Dictionary = director.advance(0.1, _world, _viewport, 11)
	var traveling: Dictionary = {"center": Vector2(3000, 3400), "zoom": entry["zoom"]}
	var entry_held: bool = true
	for step: int in range(12):
		var x: float = 3009.0 if step % 2 == 0 else 3013.0
		scene["subjects"][0]["center"] = PackedFloat64Array([x, 3500.0])
		director.set_scene(scene)
		var held: Dictionary = director.advance(0.5, _world, _viewport, 11, traveling)
		entry_held = entry_held and held["kind"] == "living" and held["center"] == entry["center"] and held["zoom"] == entry["zoom"] and held["focus"] == Vector2(x, 3500.0)
	_check(entry_held, "A moving organism shifts the approach destination or starts the hold while traveling")
	var following: Dictionary = director.advance(0.5, _world, _viewport, 0, entry)
	_check(following["kind"] == "living" and following["focus"] == Vector2(3013, 3500), "Arrival at the entry frame does not begin live follow")
	_check(director.advance(4.0, _world, _viewport, 0, entry)["kind"] == "living", "The moving subject loses its settled hold early")
	director.observe_feeding(_feeding(4000), Rect2(Vector2.ZERO, _world))
	_check(director.advance(0.5, _world, _viewport, 0, entry)["kind"] == "feeding", "A moving subject blocks a fresh feeding cut after the hold")

	director.clear()
	scene = _scene([11])
	director.set_scene(scene)
	director.advance(0.1, _world, _viewport, 11)
	scene["subjects"][0]["center"] = PackedFloat64Array([5000.0, 3500.0])
	director.set_scene(scene)
	_check(director.advance(0.5, _world, _viewport, 11, traveling)["kind"] == "scenic", "A subject outside the entry composition keeps the approach open")
