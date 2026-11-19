extends SceneTree

const Cards = preload("res://ui/living_cards.gd")
const Director = preload("res://ui/cinema_director.gd")
var _checks: int = 0
var _failures: Array[String] = []


func _check(condition: bool, message: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(message)


func _initialize() -> void:
	_run.call_deferred()


func _run() -> void:
	var simulation: BgrSimulation = BgrSimulation.new()
	_check(simulation.has_method("living_scene_snapshot"), "Load the actual new native API")
	_check(simulation.has_method("living_scene_snapshot_with_records"), "Load the additive native all-time API")
	if not simulation.has_method("living_scene_snapshot") or not simulation.has_method("living_scene_snapshot_with_records"):
		_finish()
		return
	_check(bool(simulation.preferences_initialize()["ok"]), "Initialize isolated settings")
	_check(bool(simulation.start_session(827364, 1200, 800, 32)["ok"]), "Create a real populated world")
	var before: Dictionary = simulation.snapshot()
	var scene: Dictionary = simulation.call("living_scene_snapshot", 0, true)
	_check(bool(scene.get("ok", false)) and bool(scene.get("world_ready", false)), "Bridge returns a ready typed scene")
	_check(scene["subjects"].size() > 0 and scene["subjects"].size() <= 8, "Bound the actual living subjects")
	var cards: Cards = Cards.new()
	root.add_child(cards)
	var context: Dictionary = {"population": int(before["population"]), "world_time": int(before["world_time"]), "language": "en"}
	_check(cards.set_scene(scene, context), "Real Rust arrays and record units satisfy the card contract")
	cards._next_mode = 0
	cards._next_metric = 3
	cards.advance(30.0, true)
	_check(cards.needs_previews(), "A real living mass record requests its appearance only when visible")
	var illustrated: Dictionary = simulation.call("living_scene_snapshot", 0, true)
	_check(cards.set_scene(illustrated, context), "Consume actual native appearance fields without a new API")
	var shown_token: int = cards.current_subject_token()
	var shown_image: Dictionary = {}
	for actor: Dictionary in illustrated["subjects"]:
		if actor["token"] == shown_token:
			shown_image = actor["preview"]["image"]
	_check(not shown_image.is_empty() and cards._preview._raster.get_image().get_data() == shown_image["rgb"], "The shared selection renderer displays the exact live champion RGB bytes")
	_check(not cards.needs_previews(), "Do not request a second native image for an unchanged visible champion")
	var director: Director = Director.new()
	director.set_scene(scene)
	var token: int = int(scene["subjects"][0]["token"])
	var follow: Dictionary = simulation.call("living_scene_snapshot", token, false)
	_check(int(follow["follow_token"]) == token, "The bridge resolves its live stable token")
	_check_all_time_bridge(simulation, cards, context, follow)
	_check(simulation.snapshot() == before, "Living and all-time reads leave the observable world unchanged")
	var feeding: Dictionary = follow["feeding"]
	_check(feeding["available"] and feeding["scope"] == "latest_frontend_advance_batch" and feeding["energy_unit"] == "gross_source_energy_before_costs", "Use the compiled positive-transfer observer contract")
	_check(feeding["event"].is_empty(), "Do not invent a feeding event before advancing")
	_check(bool(simulation.apply_commands(PackedInt32Array([3]))["ok"]), "Reset through the real command boundary")
	var reset: Dictionary = simulation.call("living_scene_snapshot", token, false)
	_check(reset["presentation_epoch"] > scene["presentation_epoch"] and reset["follow_token"] == 0, "Reset invalidates old identity tokens")
	_check(bool(simulation.release_world()["ok"]), "Release the actual world")
	var released: Dictionary = simulation.call("living_scene_snapshot", token, false)
	_check(released["ok"] and not released["world_ready"] and released["subjects"].is_empty(), "Released worlds retain no subjects")
	_check(not cards.set_scene(released, context), "Clear cards when the world is absent")
	var released_history: Dictionary = simulation.call("living_scene_snapshot_with_records", token, false)
	_check(released_history.get("ok", false) and not released_history.get("world_ready", true) and released_history.get("all_time_records") == [], "A released world retains no all-time records")
	cards.free()
	_check(bool(simulation.start_session(827364, 1200, 800, 512)["ok"]), "Create the fixed real-contact scenario")
	_check(bool(simulation.apply_commands(PackedInt32Array([1]))["ok"]), "Resume through the native command boundary")
	var transfer: Dictionary = {}
	for step: int in range(256):
		var advanced: Dictionary = simulation.advance(1)
		if not bool(advanced["ok"]):
			_check(false, "Real contact advancement failed")
			break
		var packet: Dictionary = advanced["feeding"]
		if not packet["event"].is_empty():
			transfer = packet
			break
	_check(not transfer.is_empty(), "Actual contact produces a positive live-peer transfer")
	if not transfer.is_empty():
		var event: Dictionary = transfer["event"]
		_check(event["predator_alive_at_transfer"] and event["prey_alive_at_transfer"] and is_finite(event["gross_energy"]) and event["gross_energy"] > 0.0, "The bridge reports actual positive gross energy")
		director.clear()
		director.observe_feeding(transfer, Rect2(Vector2.ZERO, Vector2(1200, 800)))
		_check(director.advance(0.1, Vector2(1200, 800), Vector2(1280, 800))["kind"] == "feeding", "A real native transfer activates the action camera")
		print("BGR_REAL_FEEDING " + JSON.stringify(event))
	simulation.release_world()
	_finish()


func _check_all_time_bridge(simulation: BgrSimulation, cards: Cards, context: Dictionary, baseline: Dictionary) -> void:
	var token: int = baseline["follow_token"]
	var packet: Dictionary = simulation.call("living_scene_snapshot_with_records", token, false)
	var records: Variant = packet.get("all_time_records")
	var base: Dictionary = packet.duplicate()
	base.erase("all_time_records")
	_check(base == baseline, "The additive API preserves the exact ordinary living packet")
	_check(records is Array and records.size() == 3, "The native packet has three all-time records")
	if not records is Array or records.size() != 3:
		return
	var kinds: Array[String] = ["offspring", "kills", "infections"]
	for index: int in range(kinds.size()):
		var record: Variant = records[index]
		_check(record is Dictionary and record.size() == 5 and record.get("kind") == kinds[index] and record.get("count") is int and record.get("count") == 0 and record.get("time") is int and record.get("time") == 0 and record.get("present") is bool and not record["present"] and record.get("preview") == {"requested": false, "error": "", "image": {}}, "A fresh world has a typed empty native record: " + kinds[index])
	cards.dismiss()
	cards._next_mode = 3
	cards.advance(30.0, true)
	_check(cards.needs_all_time_records() and cards.set_scene(packet, context) and not cards.needs_all_time_records() and cards.current_subject_token() == 0, "The all-time card consumes the real additive packet without a live focus token")
	var rejected: Dictionary = simulation.call("living_scene_snapshot_with_records", -1, false)
	var rejected_base: Dictionary = simulation.call("living_scene_snapshot", -1, false)
	_check(not rejected.get("ok", true) and rejected == rejected_base, "The additive API preserves base rejection semantics")


func _finish() -> void:
	print("BGR_LIVING_BRIDGE " + JSON.stringify({"checks": _checks, "failures": _failures, "result": "PASS" if _failures.is_empty() else "FAIL"}))
	quit(0 if _failures.is_empty() else 1)
