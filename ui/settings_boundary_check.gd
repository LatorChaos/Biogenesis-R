extends SceneTree

var _simulation: BgrSimulation
var _token: BgrSettingsDraft
var _checks: int = 0
var _events: int = 0
var _failed: bool = false


func _require(condition: bool, message: String) -> bool:
	_checks += 1
	if not condition:
		_failed = true
		printerr("BGR_SETTINGS_BOUNDARY_FAIL " + message)
		quit(1)
	return condition


func _controls(token: BgrSettingsDraft) -> Dictionary:
	var state: Dictionary = token.snapshot()
	return {"texts": state["texts"], "locale": state["locale"], "automatic_backup": state["automatic_backup"], "hardware": state["hardware"], "disable_framebuffer": state["disable_framebuffer"]}


func _edit(token: BgrSettingsDraft, changes: Dictionary, language: int = -1) -> void:
	var state: Dictionary = token.snapshot()
	var controls: Dictionary = _controls(token)
	var texts: PackedStringArray = controls["texts"]
	var keys: PackedStringArray = state["keys"]
	for key: String in changes:
		var index: int = keys.find(key)
		if not _require(index >= 0, "Unknown declared test editor " + key):
			return
		texts[index] = str(changes[key])
	controls["texts"] = texts
	if language >= 0:
		controls["locale"] = language
	_require(bool(token.replace_controls(controls)["ok"]), "Valid typed controls were rejected")


func _locale_changed(payload: Dictionary) -> void:
	_events += 1
	_require(str(payload["language"]) == "de", "Locale event did not precede numeric application")
	_require(int(payload["values"]["WORLD_WIDTH"]) == 1000, "Locale callback saw updated numeric width")
	_require(int(payload["values"]["MAX_MUTATION_RATE"]) == 7 and int(payload["values"]["MIN_MUTATION_RATE"]) == 7, "Locale callback lost the prior numeric pair")
	var read: Dictionary = _simulation.settings_snapshot()
	_require(bool(read["busy"]) and read["values"] == payload["values"], "Read-only reentry did not see the pre-numeric presentation")
	read["values"]["WORLD_WIDTH"] = -555
	_require(not bool(_token.defaults()["ok"]), "Reentrant draft mutation was accepted")
	var nested: Dictionary = _simulation.settings_apply(_token, _locale_changed)
	_require(not bool(nested["ok"]) and "reentrant" in str(nested["error"]), "Reentrant settings apply was not rejected explicitly")


func _same_world(before: Dictionary, after: Dictionary) -> bool:
	var first: Dictionary = before.duplicate(true)
	var second: Dictionary = after.duplicate(true)
	for key: String in ["ok", "error"]:
		first.erase(key)
		second.erase(key)
	return first == second


func _initialize() -> void:
	await process_frame
	_simulation = BgrSimulation.new()
	var original: Dictionary = _simulation.settings_snapshot()
	if not _require(original["genesis"] == PackedInt64Array([7000, 7000, 12500]), "Source startup defaults changed"):
		return
	if not _require(not (original["values"] as Dictionary).has("SEED"), "Settings acquired a session seed"):
		return
	if not _require(bool(_simulation.configure(827364, 1000, 900, 0)["ok"]), "Controlled empty world failed"):
		return
	var cancel: BgrSettingsDraft = _simulation.settings_open()
	var before_controls: Dictionary = cancel.snapshot()
	if not _require((before_controls["texts"] as PackedStringArray).size() == 132 and (before_controls["keys"] as PackedStringArray).size() == 132, "Incomplete typed editor batch"):
		return
	var malformed: Dictionary = _controls(cancel)
	malformed["texts"] = PackedStringArray(["1"])
	if not _require(not bool(cancel.replace_controls(malformed)["ok"]) and cancel.snapshot() == before_controls, "Malformed batch partially changed the draft"):
		return
	var committed: Dictionary = _simulation.settings_snapshot()
	_edit(cancel, {"WORLD_WIDTH": "4321"}, 3)
	if not _require(bool(cancel.defaults()["ok"]) and int(cancel.snapshot()["locale"]) == 3, "Defaults did not preserve the selected locale"):
		return
	if not _require(_simulation.settings_snapshot() == committed, "Draft defaults changed committed settings"):
		return
	if not _require(bool(cancel.cancel()["ok"]) and not bool(cancel.defaults()["ok"]), "Cancelled token remained writable"):
		return
	_token = _simulation.settings_open()
	_edit(_token, {"MAX_MUTATION_RATE": "7", "MIN_MUTATION_RATE": "7"})
	if not _require(bool(_simulation.settings_apply(_token, _locale_changed)["ok"]) and _events == 0, "Unchanged locale emitted an event"):
		return
	var world_before: Dictionary = _simulation.snapshot()
	_token = _simulation.settings_open()
	_edit(_token, {"WORLD_WIDTH": "1234", "WORLD_HEIGHT": "999", "INITIAL_ORGANISMS": "0", "MAX_MUTATION_RATE": "3", "MIN_MUTATION_RATE": "6", "BLUE_ENERGY_CONSUMPTION": "0.125", "INITIAL_O2": "321.75", "RUBBING": "1.5"}, 3)
	var applied: Dictionary = _simulation.settings_apply(_token, _locale_changed)
	if _failed:
		return
	if not _require(bool(applied["ok"]) and _events == 1, "Ordered apply or synchronous locale callback failed"):
		return
	if not _require(int(applied["values"]["WORLD_WIDTH"]) == 1234 and int(applied["values"]["MIN_MUTATION_RATE"]) == 7 and int(applied["values"]["MAX_MUTATION_RATE"]) == 3, "Partial source retention or read-only payload isolation failed"):
		return
	if not _require((applied["retained"] as PackedStringArray).size() == 2, "Invalid editors were not reported"):
		return
	if not _require(_same_world(world_before, _simulation.snapshot()), "Live parameter projection rebuilt observable world state"):
		return
	var rejected: Dictionary = _simulation.new_game(1234, 999, 0)
	if not _require(not bool(rejected["ok"]) and not bool(rejected["new_game_started"]) and _same_world(world_before, _simulation.snapshot()), "Unsafe retained bounds mutated the world before genesis"):
		return
	if not _require(_simulation.settings_snapshot()["genesis"] == PackedInt64Array([1234, 999, 0]), "Pre-genesis rejection changed pending settings"):
		return
	_token = _simulation.settings_open()
	_edit(_token, {"MIN_MUTATION_RATE": "3"})
	if not _require(bool(_simulation.settings_apply(_token, _locale_changed)["ok"]), "Repair through actual editors failed"):
		return
	var next: Dictionary = _simulation.new_game(1234, 999, 0)
	if not _require(bool(next["ok"]) and float(next["oxygen"]) == 321.75 and int(next["width"]) == 1234 and int(next["tick"]) == 0, "Accepted settings did not reach actual genesis"):
		return
	_simulation.release_world()
	var pending: Dictionary = _simulation.settings_snapshot()
	if not _require(float(pending["values"]["BLUE_ENERGY_CONSUMPTION"]) == 0.125 and pending["genesis"] == PackedInt64Array([1234, 999, 0]), "Stop discarded settings"):
		return
	var restarted: Dictionary = _simulation.start_session(-99, 1234, 999, 0)
	if not _require(bool(restarted["ok"]) and int(restarted["seed"]) == 827364 and float(restarted["oxygen"]) == 321.75, "Restart lost settings or changed seed provenance"):
		return
	print("BGR_SETTINGS_BOUNDARY_PASS " + str(_checks))
	quit(0)
