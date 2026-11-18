extends SceneTree

const WorldCanvas = preload("res://ui/world_canvas.gd")

var _checks: int = 0


func _initialize() -> void:
	await process_frame
	if not _crisp_check_batches():
		return
	var simulation: BgrSimulation = BgrSimulation.new()
	var initial: Dictionary = simulation.configure(827364, 1200, 800, 96)
	if not _check_state(initial):
		return
	for alpha: int in initial["alphas"]:
		if not _require(alpha == 255, "Default reference colors must remain opaque"):
			return
	if not _require(bool(initial["paused"]) and int(initial["tick"]) == 0, "Configured world must start paused at tick zero"):
		return
	if not _require(int(initial["population"]) > 0 and int(initial["population"]) <= 96, "Configured world must contain actual organisms"):
		return
	if not _require(float(initial["width"]) == 1200.0 and float(initial["height"]) == 800.0, "Configured dimensions differ"):
		return
	var observed: Dictionary = simulation.snapshot()
	if not _check_state(observed) or not _require(_same_model(initial, observed), "Snapshot changed configured state"):
		return
	var stepped: Dictionary = simulation.apply_commands(PackedInt32Array([2]))
	if not _check_state(stepped) or not _require(bool(stepped["paused"]) and int(stepped["tick"]) == 1, "Single-step did not advance one paused tick"):
		return
	var running: Dictionary = simulation.apply_commands(PackedInt32Array([1]))
	if not _check_state(running) or not _require(not bool(running["paused"]), "Resume did not change authoritative state"):
		return
	var advanced: Dictionary = simulation.advance(3)
	if not _check_state(advanced) or not _require(int(advanced["tick"]) == 4, "Explicit tick batch differs"):
		return
	var paused: Dictionary = simulation.apply_commands(PackedInt32Array([0]))
	if not _check_state(paused) or not _require(bool(paused["paused"]) and int(paused["tick"]) == 4, "Pause changed tick state"):
		return
	var invalid: Dictionary = simulation.configure(827364, -1, 800, 96)
	if not _require(not bool(invalid.get("ok", true)) and not str(invalid.get("error", "")).is_empty(), "Invalid configuration lacks an explicit error"):
		return
	if not _require(_same_model(paused, simulation.snapshot()), "Invalid configuration changed the current world"):
		return
	var invalid_command: Dictionary = simulation.apply_commands(PackedInt32Array([99]))
	if not _require(not bool(invalid_command.get("ok", true)), "Unknown command was accepted"):
		return
	if not _require(_same_model(paused, simulation.snapshot()), "Unknown command changed the current world"):
		return
	var reset: Dictionary = simulation.apply_commands(PackedInt32Array([3]))
	if not _check_state(reset) or not _require(_same_model(initial, reset), "Reset did not reproduce the seeded initial model"):
		return
	var ordered: Dictionary = simulation.apply_commands(PackedInt32Array([0, 2, 2, 1, 0]))
	if not _check_state(ordered) or not _require(bool(ordered["paused"]) and int(ordered["tick"]) == 2, "Ordered command batch changed execution order"):
		return
	var partial: Dictionary = simulation.apply_commands(PackedInt32Array([1, 2, 0]))
	if not _require(not bool(partial.get("ok", true)), "Running single-step must reject the partial batch"):
		return
	var after_error: Dictionary = simulation.snapshot()
	if not _check_state(after_error) or not _require(not bool(after_error["paused"]) and int(after_error["tick"]) == 2, "Partial batch did not stop at its first error"):
		return
	if not _check_state(simulation.apply_commands(PackedInt32Array([0]))):
		return
	for budget: int in [-1, 0, 50001, 9223372036854775807]:
		var before_budget_error: Dictionary = simulation.snapshot()
		var invalid_budget: Dictionary = simulation.advance_budgeted(16, budget)
		if not _require(not bool(invalid_budget.get("ok", true)), "Invalid simulation budget was accepted"):
			return
		if not _require(_same_model(before_budget_error, simulation.snapshot()), "Invalid budget changed the world"):
			return
	var budgeted: Dictionary = simulation.configure(827364, 1200, 800, 96)
	if not _check_state(budgeted) or not _check_state(simulation.apply_commands(PackedInt32Array([1]))):
		return
	budgeted = simulation.advance_budgeted(4096, 1)
	if not _check_state(budgeted):
		return
	var completed: int = int(budgeted["tick"])
	if not _require(completed > 0 and completed <= 4096, "Budgeted batch completed an invalid tick count"):
		return
	var exact: BgrSimulation = BgrSimulation.new()
	if not _check_state(exact.configure(827364, 1200, 800, 96)):
		return
	if not _check_state(exact.apply_commands(PackedInt32Array([1]))):
		return
	if not _require(_same_model(budgeted, exact.advance(completed)), "Yielding changed completed simulation transitions"):
		return
	if not _check_state(simulation.apply_commands(PackedInt32Array([0]))):
		return
	var before_paused_budget: Dictionary = simulation.snapshot()
	if not _require(_same_model(before_paused_budget, simulation.advance_budgeted(16, 8000)), "Budgeted advance changed a paused world"):
		return
	var empty: Dictionary = simulation.configure(42, 1200, 800, 0)
	if not _check_state(empty) or not _require(int(empty["population"]) == 0 and (empty["segments"] as PackedFloat64Array).is_empty(), "Empty initialization produced organisms"):
		return
	if not _check_player_actions():
		return
	if not _check_tracking_batches():
		return
	if not _check_laboratory():
		return
	if not _check_image_export():
		return
	if not _check_session_lifecycle():
		return
	var canvas: WorldCanvas = WorldCanvas.new()
	root.add_child(canvas)
	if not _require(canvas.is_node_ready(), "Canvas must complete ready before testing resize signals"):
		return
	canvas.size = Vector2(1000.0, 600.0)
	for dimensions: Vector2i in [Vector2i(100000000, 100000000), Vector2i(2147483647, 2147483647), Vector2i(2147483647, 1), Vector2i(1, 2147483647), Vector2i.ONE]:
		var large: Dictionary = simulation.configure(42, dimensions.x, dimensions.y, 0)
		if not _check_state(large):
			return
		canvas.set_snapshot(large["segments"], large["colors"], float(large["width"]), float(large["height"]))
		var zoom: float = float(canvas.get("_zoom"))
		var expected: float = minf(1000.0 / float(dimensions.x), 600.0 / float(dimensions.y)) * 0.92
		if not _require(is_finite(zoom) and zoom > 0.0 and absf(zoom - expected) <= expected * 0.000001, "Fit changed the independently calculated scale"):
			return
		var offset: Vector2 = canvas.get("_offset")
		if not _require(offset.x >= 0.0 and offset.y >= 0.0 and offset.x + dimensions.x * zoom <= 1000.01 and offset.y + dimensions.y * zoom <= 600.01, "Fitted world exceeds the canvas"):
			return
		canvas.zoom_at(1.2, canvas.size * 0.5)
		if not _require(is_finite(float(canvas.get("_zoom"))) and float(canvas.get("_zoom")) > zoom, "Zoom-in did not increase the fitted scale"):
			return
	canvas.size = Vector2(100.0, 100.0)
	canvas.set_snapshot(PackedFloat64Array(), PackedInt32Array(), 1000.0, 1000.0)
	canvas.set("_zoom", 1.0)
	canvas.set("_offset", Vector2(-400.0, -400.0))
	canvas.follow_organism(PackedFloat64Array([600.0, 200.0]), 5.0)
	if not _require(canvas.get("_offset") == Vector2(-410.0, -390.0), "Tracking exceeded source per-axis velocity clamp"):
		return
	canvas.follow_organism(PackedFloat64Array([900.0, 900.0]), NAN)
	if not _require(canvas.get("_offset") == Vector2(-410.0, -390.0), "NaN velocity did not follow Java's integer conversion"):
		return
	canvas.set("_offset", Vector2(-400.0, -400.0))
	canvas.follow_organism(PackedFloat64Array([600.0, 600.0]), 2147483647.0)
	if not _require(canvas.get("_offset") == Vector2(-402.0, -402.0), "Tracking changed Java overflow and Utils.between order"):
		return
	canvas.set("_zoom", 1.0)
	canvas.set("_offset", Vector2.ZERO)
	canvas.set("_fit_enabled", false)
	canvas.set_selection({"bounds": PackedInt32Array([90, 90, 5, 5]), "infected": false})
	canvas.size = Vector2(80.0, 80.0)
	var selected_center: Vector2 = canvas.get("_offset") + Vector2(92.5, 92.5)
	if not _require(Rect2(Vector2.ZERO, canvas.size).has_point(selected_center), "Inspector resize hid a previously visible selection"):
		return
	canvas.set_selection({"bounds": PackedInt32Array([200, 200, 5, 5]), "infected": false})
	var previous_offset: Vector2 = canvas.get("_offset")
	canvas.size = Vector2(60.0, 60.0)
	if not _require(canvas.get("_offset") == previous_offset, "Viewport resize jumped to an already offscreen selection"):
		return
	canvas.free()
	print("BGR_BOUNDARY_CHECK " + JSON.stringify({"result": "PASS", "checks": _checks, "scope": "actual Rust batch API; no full parity claim"}))
	quit(0)


func _check_state(state: Dictionary) -> bool:
	if not _require(bool(state.get("ok", false)), "Operation failed: " + str(state.get("error", "unspecified"))):
		return false
	for key: String in ["tick", "seed", "paused", "width", "height", "population", "oxygen", "carbon_dioxide", "segments", "colors", "alphas", "organism_ids"]:
		if not _require(state.has(key), "Missing state field: " + key):
			return false
	var segments: PackedFloat64Array = state["segments"]
	var colors: PackedInt32Array = state["colors"]
	var identities: PackedInt64Array = state["organism_ids"]
	var alphas: PackedByteArray = state["alphas"]
	if not _require(segments.size() == colors.size() * 4 and colors.size() == identities.size() and colors.size() == alphas.size(), "Segment batch shape differs"):
		return false
	if not _require(float(state["oxygen"]) >= 0.0 and float(state["carbon_dioxide"]) >= 0.0, "Atmospheric state is negative"):
		return false
	for coordinate: float in segments:
		if not _require(is_finite(coordinate), "Geometry contains a nonfinite coordinate"):
			return false
	for rgb: int in colors:
		if not _require(rgb >= 0 and rgb <= 0xffffff, "Color exceeds exact RGB range"):
			return false
	return true


func _same_session_model(left: Dictionary, right: Dictionary) -> bool:
	return _same_model(left, right) and left.get("methane") == right.get("methane") and left.get("initial_attempts") == right.get("initial_attempts")


func _check_session_lifecycle() -> bool:
	var session: BgrSimulation = BgrSimulation.new()
	var reference: BgrSimulation = BgrSimulation.new()
	var initial: Dictionary = session.start_session(827364, 1200, 800, 96)
	var seeded: Dictionary = reference.configure(827364, 1200, 800, 96)
	if not _check_state(initial) or not _require(_same_session_model(initial, seeded), "Initial session did not seed once"):
		return false
	var next: Dictionary = session.new_game(1200, 800, 96)
	var reference_next: Dictionary = reference.new_game(1200, 800, 96)
	if not _check_state(next) or not _require(bool(next["new_game_started"]) and bool(next["paused"]) and not _same_session_model(initial, next) and _same_session_model(next, reference_next), "New Game did not continue the stream reproducibly"):
		return false
	var replay: Dictionary = session.apply_commands(PackedInt32Array([3]))
	if not _require(_same_session_model(initial, replay), "Explicit Reset no longer replays the provenance seed"):
		return false
	var selected: Dictionary = _pick_actor(session)
	if selected.is_empty():
		return false
	session.copy_selection()
	var held: Dictionary = session.track_selection()
	for dimensions: PackedInt64Array in [PackedInt64Array([0, 800, 96]), PackedInt64Array([2147483648, 800, 96]), PackedInt64Array([1200, 800, 2147483648])]:
		var rejected: Dictionary = session.new_game(dimensions[0], dimensions[1], dimensions[2])
		if not _require(not bool(rejected["ok"]) and not bool(rejected["new_game_started"]) and _same_session_model(held, rejected) and held["selected"] == rejected["selected"] and bool(rejected["tracking"]) and bool(rejected["clipboard_ready"]), "Rejected New Game changed the model or frontend owners"):
			return false
	var released: Dictionary = session.release_world()
	if not _require(bool(released["ok"]) and not bool(released["world_ready"]) and bool(released["session_ready"]) and int(released["seed"]) == 827364 and not bool(released["clipboard_ready"]) and released["selected"].is_empty() and not bool(released["tracking"]) and released["genotype_preview"].is_empty(), "Stop retained the World or frontend reference owners"):
		return false
	var restarted: Dictionary = session.start_session(-7, 1200, 800, 96)
	if not _check_state(restarted) or not _require(int(restarted["seed"]) == 827364 and _same_session_model(restarted, next), "Restart reseeded or lost the detached stream"):
		return false
	var duplicate: Dictionary = session.start_session(1, 1200, 800, 96)
	if not _require(not bool(duplicate["ok"]) and _same_session_model(restarted, duplicate), "A second Start replaced the live World"):
		return false
	var changed: Dictionary = session.configure(-9223372036854775807 - 1, 1200, 800, 96)
	var changed_reference: Dictionary = reference.configure(-9223372036854775807 - 1, 1200, 800, 96)
	if not _require(_same_session_model(changed, changed_reference) and int(changed["seed"]) == -9223372036854775807 - 1, "Explicit configure no longer reseeds"):
		return false
	session.release_world()
	restarted = session.start_session(827364, 1200, 800, 0)
	if not _require(int(restarted["seed"]) == -9223372036854775807 - 1 and int(restarted["initial_attempts"]) == 0, "Restart failed to restore applied provenance and attempts"):
		return false
	var failed: BgrSimulation = BgrSimulation.new()
	var rejected_start: Dictionary = failed.start_session(827364, 0, 800, 96)
	if not _require(not bool(rejected_start["ok"]) and not bool(rejected_start["world_ready"]) and bool(rejected_start["session_ready"]), "Invalid first Start lost detached ownership"):
		return false
	failed.start_session(-1, 1200, 800, 0)
	var first_after_rejection: Dictionary = failed.new_game(1200, 800, 96)
	if not _require(_same_session_model(initial, first_after_rejection), "Rejected Start consumed or reseeded the stream"):
		return false
	var partial: BgrSimulation = BgrSimulation.new()
	var construction_error: Dictionary = partial.start_session(827364, 1, 1, 1)
	if not _require(not bool(construction_error["ok"]) and not bool(construction_error["world_ready"]) and bool(construction_error["session_ready"]) and int(construction_error["seed"]) == 827364, "Construction failure lost its actual remaining stream"):
		return false
	partial.release_world()
	var recovered: Dictionary = partial.start_session(-9, 1200, 800, 96)
	return _check_state(recovered) and _require(int(recovered["seed"]) == 827364 and not _same_session_model(initial, recovered), "Retry discarded construction's consumed draws")


func _same_model(left: Dictionary, right: Dictionary) -> bool:
	for key: String in ["tick", "seed", "paused", "width", "height", "population", "oxygen", "carbon_dioxide", "segments", "colors", "alphas", "organism_ids"]:
		if left.get(key) != right.get(key):
			return false
	return true


func _require(condition: bool, message: String) -> bool:
	_checks += 1
	if not condition:
		push_error(message)
		print("BGR_BOUNDARY_CHECK " + JSON.stringify({"result": "FAIL", "message": message, "checks": _checks}))
		quit(1)
	return condition


func _pick_actor(simulation: BgrSimulation, skip_id: int = -1) -> Dictionary:
	var state: Dictionary = simulation.snapshot()
	var points: PackedFloat64Array = state["segments"]
	var identities: PackedInt64Array = state["organism_ids"]
	for index: int in range(identities.size()):
		if identities[index] == skip_id:
			continue
		var result: Dictionary = simulation.select_at(floor(points[index * 4]), floor(points[index * 4 + 1]), false)
		var selected: Dictionary = result["selected"]
		if not selected.is_empty() and int(selected["id"]) != skip_id:
			return result
	_require(false, "Cannot select an actual seeded organism")
	return {}


func _check_player_actions() -> bool:
	var simulation: BgrSimulation = BgrSimulation.new()
	var initial: Dictionary = simulation.configure(827364, 1200, 800, 96)
	if not _require(bool(initial["ok"]) and initial["selected"].is_empty() and not bool(initial["tracking"]), "New world retained a selection or tracking handle"):
		return false
	var selected: Dictionary = _pick_actor(simulation)
	if selected.is_empty():
		return false
	var first: Dictionary = selected["selected"]
	var id: int = first["id"]
	if not _require(bool(selected["selection_hit"]) and bool(first["alive"]) and first["bounds"].size() == 4, "Picking did not return a living member and current bounds"):
		return false
	var tracked: Dictionary = simulation.track_selection()
	if not _require(bool(tracked["ok"]) and bool(tracked["tracking"]) and bool(tracked["selected_is_tracked"]) and tracked["tracking_center"].size() == 2, "Tracking did not retain the selected identity"):
		return false
	selected = _pick_actor(simulation, id)
	if selected.is_empty():
		return false
	if not _require(bool(selected["tracking"]) and not bool(selected["selected_is_tracked"]), "Changing selection changed the independent tracked identity"):
		return false
	var preserved: Dictionary = simulation.select_at(-100.0, -100.0, true)
	if not _require(not bool(preserved["selection_hit"]) and preserved["selected"] == selected["selected"], "Empty right click changed selection or reported a hit"):
		return false
	var before_invalid: Dictionary = simulation.snapshot()
	for point: Vector2 in [Vector2(INF, 0.0), Vector2(0.0, NAN), Vector2(2147483648.0, 0.0)]:
		var invalid: Dictionary = simulation.select_at(point.x, point.y, false)
		if not _require(not bool(invalid["ok"]) and invalid["selected"] == before_invalid["selected"] and bool(invalid["tracking"]), "Invalid coordinates changed selection or tracking"):
			return false
	var before: Dictionary = simulation.snapshot()
	var amount: float = minf(10.0, float(before["carbon_dioxide"]))
	var fed: Dictionary = simulation.organism_action("feed")
	if not _require(bool(fed["ok"]) and bool(fed["action_applied"]) and float(fed["selected"]["energy"]) == float(before["selected"]["energy"]) + amount and float(fed["carbon_dioxide"]) == float(before["carbon_dioxide"]) + amount and float(fed["oxygen"]) == float(before["oxygen"]) + amount and fed["tick"] == before["tick"], "Feed changed source gas/energy order or advanced a tick"):
		return false
	var weakened: Dictionary = simulation.organism_action("weaken")
	var half: float = float(fed["selected"]["energy"]) / 2.0
	if not _require(bool(weakened["ok"]) and float(weakened["selected"]["energy"]) == float(fed["selected"]["energy"]) - half and float(weakened["oxygen"]) == float(fed["oxygen"]) - half and float(weakened["carbon_dioxide"]) == float(fed["carbon_dioxide"]) + half, "Weaken did not pay through actual atmosphere accounting"):
		return false
	var killed: Dictionary = simulation.organism_action("kill")
	if not _require(bool(killed["ok"]) and not bool(killed["selected"]["alive"]) and int(killed["population"]) == int(weakened["population"]) - 1 and killed["selected"]["energy"] == weakened["selected"]["energy"], "Kill lost the corpse or changed its energy"):
		return false
	for action: String in ["feed", "weaken", "kill", "reproduce", "rejuvenate"]:
		var rejected: Dictionary = simulation.organism_action(action)
		if not _require(bool(rejected["ok"]) and not bool(rejected["action_applied"]) and rejected["selected"] == killed["selected"] and rejected["population"] == killed["population"], "Dead organism accepted " + action):
			return false
	var revived: Dictionary = simulation.organism_action("revive")
	if not _require(bool(revived["ok"]) and bool(revived["selected"]["alive"]) and int(revived["selected"]["age_ticks"]) == 1 and revived["selected"]["energy"] == killed["selected"]["energy"] and int(revived["population"]) == int(killed["population"]) + 1, "Revive lost source age, energy, or population behavior"):
		return false
	var rejuvenated: Dictionary = simulation.organism_action("rejuvenate")
	if not _require(bool(rejuvenated["ok"]) and int(rejuvenated["selected"]["age_ticks"]) == 0, "Rejuvenate did not reset the selected age"):
		return false
	for action: String in ["revive", "disperse"]:
		var rejected: Dictionary = simulation.organism_action(action)
		if not _require(bool(rejected["ok"]) and not bool(rejected["action_applied"]) and rejected["selected"] == rejuvenated["selected"], "Living organism accepted " + action):
			return false
	var invalid_action: Dictionary = simulation.organism_action("Feed")
	if not _require(not bool(invalid_action["ok"]) and invalid_action["selected"] == rejuvenated["selected"], "Invalid action changed the model"):
		return false
	var reproduced: Dictionary = simulation.organism_action("reproduce")
	if not _require(bool(reproduced["ok"]) and bool(reproduced["action_applied"]) and reproduced["tick"] == rejuvenated["tick"], "Reproduce failed to call the real selected callback while paused"):
		return false
	var cleared: Dictionary = simulation.select_at(-100.0, -100.0, false)
	if not _require(cleared["selected"].is_empty() and bool(cleared["tracking"]), "Empty left click did not clear selection independently"):
		return false
	var no_selection: Dictionary = simulation.organism_action("kill")
	if not _require(not bool(no_selection["ok"]), "An action without selection succeeded"):
		return false
	if not _require(not bool(simulation.abort_tracking()["tracking"]), "Stop tracking retained its handle"):
		return false
	before = simulation.snapshot()
	var invalid_batch: Dictionary = simulation.apply_commands(PackedInt32Array([6, 10, 7]))
	if not _require(not bool(invalid_batch["ok"]) and invalid_batch["carbon_dioxide"] == before["carbon_dioxide"], "Invalid command batch partially changed gas"):
		return false
	var gas: Dictionary = simulation.apply_commands(PackedInt32Array([6, 8]))
	if not _require(bool(gas["ok"]) and float(gas["carbon_dioxide"]) == float(before["carbon_dioxide"]) + 1000.0 and float(gas["methane"]) == float(before["methane"]) + 1000.0, "World gas menu commands differ"):
		return false
	gas = simulation.apply_commands(PackedInt32Array([7, 9]))
	if not _require(bool(gas["ok"]) and gas["carbon_dioxide"] == before["carbon_dioxide"] and gas["methane"] == before["methane"], "World gas removal did not match source decrement"):
		return false
	if not _require(int(simulation.apply_commands(PackedInt32Array([4]))["population"]) == 0, "Kill all retained living members"):
		return false
	selected = _pick_actor(simulation)
	if selected.is_empty():
		return false
	var dispersed: Dictionary = simulation.organism_action("disperse")
	if not _require(bool(dispersed["ok"]) and float(dispersed["selected"]["energy"]) == 0.0, "Disperse did not consume this corpse's energy"):
		return false
	var all_dispersed: Dictionary = simulation.apply_commands(PackedInt32Array([5]))
	if not _require(bool(all_dispersed["ok"]), "Disperse all failed"):
		return false
	var removed: Dictionary = simulation.apply_commands(PackedInt32Array([2]))
	if not _require(bool(removed["ok"]) and removed["selected"].is_empty() and not bool(removed["tracking"]) and removed["segments"].is_empty(), "Actual next frame retained dispersed members or selection"):
		return false
	var reset: Dictionary = simulation.apply_commands(PackedInt32Array([3]))
	return _require(bool(reset["ok"]) and reset["selected"].is_empty() and not bool(reset["tracking"]) and int(reset["population"]) == int(initial["population"]), "Reset did not restore the world and clear handles")


func _check_tracking_batches() -> bool:
	var batch: BgrSimulation = BgrSimulation.new()
	var individual: BgrSimulation = BgrSimulation.new()
	for simulation: BgrSimulation in [batch, individual]:
		simulation.configure(827364, 1200, 800, 96)
		if _pick_actor(simulation).is_empty():
			return false
		var tracked: Dictionary = simulation.track_selection()
		if not _require(tracked["tracking_path"].is_empty(), "Tracking moved the paused camera"):
			return false
		simulation.apply_commands(PackedInt32Array([1]))
	var advanced: Dictionary = batch.advance(4)
	var expected: PackedInt32Array = PackedInt32Array()
	var one: Dictionary = {}
	for _tick: int in range(4):
		one = individual.advance(1)
		var center: PackedFloat64Array = one["tracking_center"]
		expected.append(int(center[0]))
		expected.append(int(center[1]))
	if not _require(advanced["tracking_path"] == expected and expected.size() == 8 and _same_model(advanced, one), "Batched tracking omitted actual intermediate centers or changed the model"):
		return false
	batch.apply_commands(PackedInt32Array([0]))
	for response: Dictionary in [batch.snapshot(), batch.select_at(-100.0, -100.0, true), batch.organism_action("feed"), batch.advance(4)]:
		if not _require(response["tracking_path"].is_empty(), "Paused UI action moved the camera"):
			return false
	var partial: Dictionary = batch.apply_commands(PackedInt32Array([2, 1, 2]))
	return _require(not bool(partial["ok"]) and int(partial["tick"]) == 5 and partial["tracking_path"].size() == 2, "Partial batch lost the completed tick's camera update")


func _draft_state(simulation: BgrSimulation, draft: BgrGenomeDraft) -> Dictionary:
	return simulation.laboratory_snapshot(draft, 180, 180)


func _check_laboratory() -> bool:
	var simulation: BgrSimulation = BgrSimulation.new()
	var initial: Dictionary = simulation.configure(827364, 1200, 800, 0)
	var opened: Dictionary = simulation.open_laboratory("empty")
	if not _require(bool(opened["ok"]), "Empty laboratory did not open"):
		return false
	var draft: BgrGenomeDraft = opened["draft"]
	var empty: Dictionary = _draft_state(simulation, draft)
	if not _require(empty["genes"].is_empty() and empty["properties"]["symmetry"] == 2 and empty["properties"]["mutation_rate"] == 50 and not bool(initial["clipboard_ready"]), "Empty laboratory source defaults differ"):
		return false
	if not _require(bool(draft.edit_rows("insert", 0)["ok"]), "Cannot append the source default gene"):
		return false
	var state: Dictionary = _draft_state(simulation, draft)
	var gene: Dictionary = state["genes"][0]
	if not _require(gene["length"] == 2.0 and gene["theta"] == 0.0 and gene["color"] == 0xff00ff00 and gene["branch"] == -1 and gene["reactions"] == PackedInt32Array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]), "Add gene changed fixed source values"):
		return false
	var reaction_names: Array[String] = ["red", "green", "blue", "plague", "scourge", "white", "gray", "silver", "default", "consumer", "plant", "magenta", "pink", "coral", "orange", "bark", "violet", "virus", "maroon", "olive", "mint", "cream", "spike", "fallow", "lightblue", "ochre", "sky", "lilac", "fire", "lightbrown", "greenbrown", "brown", "ice", "broken", "sick", "friend"]
	var expected_reactions: PackedInt32Array = gene["reactions"].duplicate()
	for index: int in reaction_names.size():
		var value: int = index % 5 + 1
		if not _require(bool(draft.set_gene_value(0, "reaction_" + reaction_names[index], value)["ok"]), "Cannot edit reaction " + reaction_names[index]):
			return false
		expected_reactions[index] = value
		if not _require(_draft_state(simulation, draft)["genes"][0]["reactions"] == expected_reactions, "Reaction edit changed another channel"):
			return false
	for field: String in ["plague", "disperse_children", "generation_battle", "sibling_battle", "altruist", "familial", "social", "peaceful", "passive", "clockwise", "mimic_all", "modifies_pink", "modifies_lilac", "modifies_sky", "selfish"]:
		if not _require(bool(simulation.laboratory_set_property(draft, field, true)["ok"]) and bool(_draft_state(simulation, draft)["properties"][field]), "Boolean metadata edit failed: " + field):
			return false
	for entry: Array in [["symmetry", 1], ["mirror", 1], ["mutation_rate", 27], ["activity", 0], ["cream", 3], ["fallow", 4], ["spore", 12], ["adapt_spore", 70]]:
		if not _require(bool(simulation.laboratory_set_property(draft, entry[0], entry[1])["ok"]) and _draft_state(simulation, draft)["properties"][entry[0]] == entry[1], "Numeric metadata edit failed"):
			return false
	if not _require(bool(draft.set_gene_value(0, "length", 12.125)["ok"]) and bool(draft.step_gene_value(0, "length", 1)["ok"]) and _draft_state(simulation, draft)["genes"][0]["length"] == 12.125 + 0.1, "Length stepping changed raw source precision"):
		return false
	if not _require(bool(draft.set_gene_value(0, "angle_degrees", 359)["ok"]) and bool(draft.step_gene_value(0, "angle_degrees", 1)["ok"]) and _draft_state(simulation, draft)["genes"][0]["theta"] == 0.0, "Angle step failed to wrap"):
		return false
	if not _require(bool(draft.set_gene_value(0, "palette_color", 0)["ok"]) and _draft_state(simulation, draft)["genes"][0]["color"] == 0xff00ff00, "Named palette selection did not assign its source color"):
		return false
	if not _require(bool(draft.set_gene_value(0, "color", 0x80ff0000)["ok"]) and _draft_state(simulation, draft)["genes"][0]["color"] == 0x80ff0000, "Full ARGB editor value was lost"):
		return false
	var before_invalid: Dictionary = _draft_state(simulation, draft)
	for rejected: Dictionary in [simulation.laboratory_set_property(draft, "symmetry", 9), simulation.laboratory_set_property(draft, "plague", 1), simulation.laboratory_set_property(draft, "mirror", 0.0), simulation.laboratory_set_property(draft, "unknown", 1), draft.set_gene_value(0, "length", NAN), draft.set_gene_value(0, "branch", 1000), draft.set_gene_value(0, "angle_degrees", -1), draft.set_gene_value(0, "reaction_green", 6), draft.set_gene_value(0, "color", 4294967296), draft.set_gene_value(0, "palette_color", -1), draft.set_gene_value(0, "palette_color", 64), draft.set_gene_value(0, "palette_color", 0.0), draft.set_gene_value(0, "palette_color", "0"), draft.set_gene_value(-1, "length", 3.0), draft.edit_rows("insert", 10)]:
		if not _require(not bool(rejected["ok"]) and _draft_state(simulation, draft) == before_invalid, "Invalid editor input mutated a draft"):
			return false
	if not _require(_same_model(initial, simulation.snapshot()), "Pure draft editing changed the actual world"):
		return false
	var committed: Dictionary = simulation.commit_laboratory(draft)
	if not _require(bool(committed["ok"]) and bool(committed["clipboard_ready"]) and not bool(draft.edit_rows("clear", 0)["ok"]), "Commit did not consume only its own draft"):
		return false
	var second: BgrGenomeDraft = simulation.open_laboratory("clipboard")["draft"]
	if not _require(_draft_state(simulation, second)["genes"][0]["reactions"] == expected_reactions, "Clipboard commit omitted reactions"):
		return false
	var placed: Dictionary = simulation.paste_clipboard(600.0, 400.0)
	if not _require(bool(placed["ok"]) and bool(placed["placement_succeeded"]) and int(placed["population"]) == 1 and placed["alphas"][0] == 128, "Clipboard paste failed or removed alpha"):
		return false
	var picked: Dictionary = _pick_actor(simulation)
	if picked.is_empty() or not _require(bool(simulation.copy_selection()["ok"]), "Living genotype copy failed"):
		return false
	var directory: String = _boundary_output_root().path_join("bgr-genotype-boundary")
	if not _require(directory.is_absolute_path() and DirAccess.make_dir_recursive_absolute(directory) == OK, "Cannot create owned genotype test folder"):
		return false
	var path: String = directory.path_join("actual.bgg")
	var alpha_export: Dictionary = simulation.export_selection(path)
	if not _require(not bool(alpha_export["ok"]) and str(alpha_export["error"]).contains("legacy XML color name") and not bool(simulation.genotype_destination(path)["exists"]), "Unsupported legacy XML color did not fail before writing"):
		return false
	var legacy: BgrGenomeDraft = simulation.open_laboratory("clipboard")["draft"]
	if not _require(bool(legacy.set_gene_value(0, "color", 0xffff0000)["ok"]) and bool(simulation.commit_laboratory(legacy)["ok"]) and bool(simulation.paste_clipboard(900.0, 400.0)["placement_succeeded"]), "Cannot prepare a legacy-representable actual organism"):
		return false
	if _pick_actor(simulation, int(picked["selected"]["id"])).is_empty():
		return false
	var exported: Dictionary = simulation.export_selection(path)
	if not _require(bool(exported["ok"]) and bool(simulation.genotype_destination(path)["exists"]), "Actual genotype export failed: " + str(exported["error"])):
		return false
	var imported: BgrGenomeDraft = simulation.open_laboratory("empty")["draft"]
	if not _require(bool(imported.import_file(path)["ok"]) and _draft_state(simulation, imported)["genes"][0]["color"] == 0xffff0000 and _draft_state(simulation, imported)["genes"][0]["reactions"] == expected_reactions, "File readback lost genome fields"):
		return false
	var written: String = FileAccess.get_file_as_string(path)
	var old_length: String = written.get_slice('length="', 1).get_slice('"', 0)
	var raw_path: String = directory.path_join("raw.bgg")
	var raw_file: FileAccess = FileAccess.open(raw_path, FileAccess.WRITE)
	if not _require(raw_file != null and not old_length.is_empty(), "Cannot prepare bounded raw genotype import"):
		return false
	raw_file.store_string(written.replace('length="' + old_length + '"', 'length="19.25"'))
	raw_file.close()
	if not _require(bool(imported.import_file(raw_path)["ok"]) and _draft_state(simulation, imported)["genes"][0]["length"] == 19.25 and not _draft_state(simulation, imported)["editor_warnings"].is_empty(), "Raw import did not preserve and identify invalid editor values"):
		return false
	if not _require(bool(imported.set_gene_value(0, "length", 18.0)["ok"]) and _draft_state(simulation, imported)["editor_warnings"].is_empty(), "Raw imported value cannot be corrected"):
		return false
	var invalid_file: FileAccess = FileAccess.open(directory.path_join("invalid.bgg"), FileAccess.WRITE)
	if not _require(invalid_file != null, "Cannot create owned malformed genotype"):
		return false
	invalid_file.store_string("<invalid>")
	invalid_file.close()
	var before_import: Dictionary = _draft_state(simulation, imported)
	if not _require(not bool(imported.import_file(directory.path_join("invalid.bgg"))["ok"]) and _draft_state(simulation, imported) == before_import, "Malformed lab import replaced the draft"):
		return false
	if not _require(not bool(simulation.genotype_destination(directory)["ok"]), "Directory accepted as genotype destination"):
		return false
	var killed: Dictionary = simulation.organism_action("kill")
	if not _require(bool(killed["ok"]) and not bool(simulation.copy_selection()["ok"]) and bool(simulation.open_laboratory("selection")["ok"]) and bool(simulation.copy_preview()["ok"]), "Corpse examine/preview-copy guards differ"):
		return false
	var replacement: BgrGenomeDraft = simulation.open_laboratory("empty")["draft"]
	if not _require(bool(replacement.edit_rows("insert", 0)["ok"]) and bool(replacement.set_gene_value(0, "length", 7.5)["ok"]) and bool(simulation.commit_laboratory(replacement)["ok"]), "Cannot prepare a distinct replacement clipboard"):
		return false
	if not _require(bool(simulation.copy_preview()["ok"]) and _draft_state(simulation, simulation.open_laboratory("clipboard")["draft"])["genes"][0]["color"] == 0xffff0000, "Visible corpse preview did not copy its actual genotype"):
		return false
	replacement = simulation.open_laboratory("empty")["draft"]
	if not _require(bool(replacement.edit_rows("insert", 0)["ok"]) and bool(replacement.set_gene_value(0, "length", 7.5)["ok"]) and bool(simulation.commit_laboratory(replacement)["ok"]), "Cannot replace clipboard before clearing the preview"):
		return false
	simulation.select_at(-100.0, -100.0, false)
	var hidden_copy: Dictionary = simulation.copy_preview()
	var retained: BgrGenomeDraft = simulation.open_laboratory("clipboard")["draft"]
	if not _require(bool(hidden_copy["ok"]) and _draft_state(simulation, retained)["genes"][0]["length"] == 7.5 and _draft_state(simulation, retained)["genes"][0]["color"] == 0xff00ff00, "An absent preview replaced the newer clipboard with a hidden old genotype"):
		return false
	retained.cancel()
	var empty_commit: BgrGenomeDraft = simulation.open_laboratory("empty")["draft"]
	if not _require(bool(simulation.commit_laboratory(empty_commit)["clipboard_ready"]), "Empty commit cleared the clipboard"):
		return false
	if not _require(bool(simulation.configure(3, 1200, 800, 0)["clipboard_ready"]) and bool(simulation.apply_commands(PackedInt32Array([3]))["clipboard_ready"]), "World replacement or reset cleared clipboard identity"):
		return false
	second.cancel()
	imported.cancel()
	return _require(not bool(second.edit_rows("clear", 0)["ok"]), "Canceled draft remained writable")


func _check_image_export() -> bool:
	var simulation: BgrSimulation = BgrSimulation.new()
	simulation.configure(827364, 1200, 800, 0)
	if not _require(not bool(simulation.prepare_selection_image()["ok"]), "An absent selection produced an image"):
		return false
	var draft: BgrGenomeDraft = simulation.open_laboratory("empty")["draft"]
	if not _require(bool(draft.edit_rows("insert", 0)["ok"]) and bool(simulation.laboratory_set_property(draft, "symmetry", 1)["ok"]) and bool(draft.set_gene_value(0, "length", 12.0)["ok"]) and bool(draft.set_gene_value(0, "color", 0x80ff0000)["ok"]) and bool(simulation.commit_laboratory(draft)["ok"]), "Cannot prepare an actual translucent image organism"):
		return false
	if not _require(bool(simulation.paste_clipboard(600.0, 400.0)["placement_succeeded"]), "Cannot place the image organism"):
		return false
	if _pick_actor(simulation).is_empty():
		return false
	var before: Dictionary = simulation.snapshot()
	var prepared: Dictionary = simulation.prepare_selection_image()
	if not _require(bool(prepared["ok"]) and _same_model(before, simulation.snapshot()), "Image preparation failed or changed the actual world"):
		return false
	var image_export: BgrImageExport = prepared["export"]
	var raw: Dictionary = image_export.snapshot()
	var expected_rgb: PackedByteArray = raw["rgb"].duplicate()
	var has_translucent_red: bool = false
	var rgb: PackedByteArray = raw["rgb"]
	for index: int in range(0, rgb.size(), 3):
		if rgb[index] == 128 and rgb[index + 1] == 0 and rgb[index + 2] == 0:
			has_translucent_red = true
	if not _require(has_translucent_red, "Translucent phenotype did not blend onto opaque black"):
		return false
	var directory: String = _boundary_output_root().path_join("image-boundary")
	if not _require(directory.is_absolute_path() and DirAccess.make_dir_recursive_absolute(directory) == OK, "Cannot create the isolated PNG directory"):
		return false
	var path: String = directory.path_join("organism.png")
	raw["rgb"] = PackedByteArray([137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130])
	if not _require(bool(image_export.save_to_file(path)["ok"]), "Cannot save the actual frozen image"):
		return false
	var saved: PackedByteArray = FileAccess.get_file_as_bytes(path)
	var restored: Image = Image.new()
	if not _require(restored.load_png_from_buffer(saved) == OK, "Actual PNG file cannot be decoded"):
		return false
	restored.convert(Image.FORMAT_RGB8)
	if not _require(restored.get_width() == raw["width"] and restored.get_height() == raw["height"] and restored.get_data() == expected_rgb, "PNG encoding or external snapshot mutation changed any frozen phenotype pixel"):
		return false
	var empty_export: BgrImageExport = BgrImageExport.new()
	if not _require(not bool(empty_export.save_to_file(path)["ok"]) and FileAccess.get_file_as_bytes(path) == saved, "An empty export object changed an existing file"):
		return false
	if not _require(not bool(image_export.save_to_file(directory)["ok"]), "A directory was accepted as a PNG destination"):
		return false
	simulation.organism_action("kill")
	if not _require(not bool(simulation.prepare_selection_image()["ok"]), "A corpse produced a selection export image"):
		return false
	simulation.organism_action("disperse")
	if not _require(not bool(simulation.prepare_selection_image()["ok"]), "A removed selection produced an image"):
		return false
	if not _require(bool(image_export.save_to_file(path)["ok"]) and FileAccess.get_file_as_bytes(path) == saved, "Removing the source organism changed its frozen export"):
		return false
	return true


func _crisp_check_batches() -> bool:
	return _crisp_check_opaque_batches() and _crisp_check_ineligible() and _crisp_check_invalid() and _crisp_check_parent_guards()


# Frozen opaque branch from the original WorldCanvas renderer
func _crisp_original_opaque_batch(segments: PackedFloat64Array, colors: PackedInt32Array, alphas: PackedByteArray, zoom: float, offset: Vector2) -> Dictionary:
	var batch_points: PackedVector2Array = PackedVector2Array()
	var batch_colors: PackedColorArray = PackedColorArray()
	for index: int in range(colors.size()):
		var start: int = index * 4
		var from_point: Vector2 = Vector2(segments[start] * zoom + offset.x, segments[start + 1] * zoom + offset.y)
		var to_point: Vector2 = Vector2(segments[start + 2] * zoom + offset.x, segments[start + 3] * zoom + offset.y)
		var rgb: int = colors[index]
		var color: Color = Color(float((rgb >> 16) & 255) / 255.0, float((rgb >> 8) & 255) / 255.0, float(rgb & 255) / 255.0)
		if not alphas.is_empty():
			color.a = float(alphas[index]) / 255.0
		if from_point == to_point:
			batch_points.append(from_point - Vector2(0.5, 0.0))
			batch_points.append(from_point + Vector2(0.5, 0.0))
		else:
			batch_points.append(from_point)
			batch_points.append(to_point)
		batch_colors.append(color)
	return {"points": batch_points, "colors": batch_colors}


func _crisp_check_batch(segments: PackedFloat64Array, colors: PackedInt32Array, alphas: PackedByteArray, zoom: float, offset: Vector2) -> bool:
	var actual: Dictionary = BgrSimulation.prepare_crisp_batch(segments, colors, alphas, zoom, offset)
	if not _require(bool(actual.get("ok", false)) and bool(actual.get("eligible", false)) and str(actual.get("error", "missing")).is_empty(), "Valid opaque batch was rejected"):
		return false
	var expected: Dictionary = _crisp_original_opaque_batch(segments, colors, alphas, zoom, offset)
	var actual_points: PackedVector2Array = actual["points"]
	var expected_points: PackedVector2Array = expected["points"]
	var actual_colors: PackedColorArray = actual["colors"]
	var expected_colors: PackedColorArray = expected["colors"]
	if not _require(actual_points.to_byte_array() == expected_points.to_byte_array(), "Packed line points differ from the original renderer at zoom %s offset %s" % [zoom, offset]):
		return false
	return _require(actual_colors.to_byte_array() == expected_colors.to_byte_array(), "Packed line colors differ from the original renderer")


func _crisp_check_opaque_batches() -> bool:
	var segments: PackedFloat64Array = PackedFloat64Array([
		0.0, 0.0, 0.0, 0.0,
		-0.0, -0.0, 0.0, -0.0,
		16777216.0, -16777216.0, 16777217.0, -16777217.0,
		0.123456789, -0.987654321, 0.1234567885, -0.9876543205,
		2147483647.0, -2147483648.0, -2147483648.0, 2147483647.0,
		1e-300, -1e-300, 2e-300, -2e-300,
		1e20, -1e20, -1e20, 1e20,
	])
	var colors: PackedInt32Array = PackedInt32Array([0, 0xffffff, -1, -2147483648, 0x112233, -2146360781, 0xabcdef])
	for value: int in range(256):
		var x: float = float(value) / 7.0
		var y: float = -float(value) / 11.0
		segments.append_array(PackedFloat64Array([x, y, x if value % 3 == 0 else x + 0.125, y if value % 3 == 0 else y - 0.0625]))
		colors.append((value << 16) | ((255 - value) << 8) | (value ^ 0x5a))
	var opaque: PackedByteArray = PackedByteArray()
	opaque.resize(colors.size())
	opaque.fill(255)
	for zoom: float in [1e-12, 0.076, 1.0, 1.2, 1e6]:
		for offset: Vector2 in [Vector2.ZERO, Vector2(-123.125, 842.5), Vector2(16777216.0, -16777216.0), Vector2(0.5, -0.0)]:
			if not _crisp_check_batch(segments, colors, PackedByteArray(), zoom, offset) or not _crisp_check_batch(segments, colors, opaque, zoom, offset):
				return false
	return _crisp_check_batch(PackedFloat64Array(), PackedInt32Array(), PackedByteArray(), 1.0, Vector2.ZERO)


func _crisp_check_ineligible() -> bool:
	for alpha: int in [0, 1, 254]:
		var result: Dictionary = BgrSimulation.prepare_crisp_batch(PackedFloat64Array([0.0, 0.0, 1.0, 1.0, 2.0, 3.0, 2.0, 3.0]), PackedInt32Array([0xffffff, 0x102030]), PackedByteArray([255, alpha]), 1.0, Vector2.ZERO)
		if not _require(bool(result["ok"]) and not bool(result["eligible"]) and str(result["error"]).is_empty() and result["points"].is_empty() and result["colors"].is_empty(), "Translucent batch did not select the unchanged fallback"):
			return false
	return true


func _crisp_rejected(segments: PackedFloat64Array, colors: PackedInt32Array, alphas: PackedByteArray, zoom: float, offset: Vector2) -> bool:
	var result: Dictionary = BgrSimulation.prepare_crisp_batch(segments, colors, alphas, zoom, offset)
	return _require(not bool(result.get("ok", true)) and not bool(result.get("eligible", true)) and not str(result.get("error", "")).is_empty() and result["points"].is_empty() and result["colors"].is_empty(), "Invalid batch did not fail explicitly without partial output")


func _crisp_check_invalid() -> bool:
	var segments: PackedFloat64Array = PackedFloat64Array([0.0, 0.0, 1.0, 1.0])
	var colors: PackedInt32Array = PackedInt32Array([0xffffff])
	if not _crisp_rejected(PackedFloat64Array([0.0, 0.0, 1.0]), colors, PackedByteArray(), 1.0, Vector2.ZERO):
		return false
	if not _crisp_rejected(segments, PackedInt32Array(), PackedByteArray(), 1.0, Vector2.ZERO):
		return false
	if not _crisp_rejected(segments, colors, PackedByteArray([255, 255]), 1.0, Vector2.ZERO):
		return false
	for zoom: float in [0.0, -1.0, NAN, INF, -INF]:
		if not _crisp_rejected(segments, colors, PackedByteArray(), zoom, Vector2.ZERO):
			return false
	for offset: Vector2 in [Vector2(NAN, 0.0), Vector2(0.0, INF)]:
		if not _crisp_rejected(segments, colors, PackedByteArray(), 1.0, offset):
			return false
	for value: float in [NAN, INF, 1e300]:
		if not _crisp_rejected(PackedFloat64Array([value, 0.0, 1.0, 1.0]), colors, PackedByteArray(), 1.0, Vector2.ZERO):
			return false
	return true


func _crisp_guard_ineligible(canvas: WorldCanvas) -> bool:
	var result: Dictionary = canvas._prepare_crisp_batch()
	return _require(bool(result["ok"]) and not bool(result["eligible"]), "Canvas parent/material guard did not retain the original fallback")


func _crisp_check_parent_guards() -> bool:
	var parent: Control = Control.new()
	var canvas: WorldCanvas = WorldCanvas.new()
	root.add_child(parent)
	parent.add_child(canvas)
	canvas.set_snapshot(PackedFloat64Array([0.0, 0.0, 1.0, 1.0]), PackedInt32Array([0xffffff]), 100.0, 100.0)
	if not _require(bool(canvas._prepare_crisp_batch()["eligible"]), "Unmodified opaque canvas cannot use the batch"):
		return false
	# Invalid geometry proves the parent guard prevents the Rust call
	canvas.set_snapshot(PackedFloat64Array([NAN, 0.0, 1.0, 1.0]), PackedInt32Array([0xffffff]), 100.0, 100.0)
	canvas.self_modulate = Color(1.0, 1.0, 1.0, 0.5)
	if not _crisp_guard_ineligible(canvas):
		return false
	canvas.self_modulate = Color.WHITE
	parent.modulate = Color(0.5, 1.0, 1.0, 1.0)
	if not _crisp_guard_ineligible(canvas):
		return false
	parent.modulate = Color.WHITE
	parent.material = CanvasItemMaterial.new()
	if not _crisp_guard_ineligible(canvas):
		return false
	parent.material = null
	canvas.material = CanvasItemMaterial.new()
	if not _crisp_guard_ineligible(canvas):
		return false
	canvas.material = null
	canvas.use_parent_material = true
	if not _crisp_guard_ineligible(canvas):
		return false
	parent.free()
	return true


func _boundary_output_root() -> String:
	var owned: String = OS.get_environment("BGR_TEST_ROOT")
	if not owned.is_empty():
		return owned
	return OS.get_user_data_dir().path_join("boundary-%d" % OS.get_process_id())
