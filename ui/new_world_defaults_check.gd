extends SceneTree

var _checks: int = 0
var _failures: Array[String] = []
var _settings_diff: Array[Dictionary] = []
var _co2_types: Dictionary = {}
var _launcher: Control


func _check(value: bool, message: String) -> bool:
	_checks += 1
	if not value:
		_failures.append(message)
	return value


func _initialize() -> void:
	_run.call_deferred()


func _write(path: String, body: String) -> bool:
	var stream: FileAccess = FileAccess.open(path, FileAccess.WRITE)
	if not _check(stream != null, "Open the isolated preference fixture"):
		return false
	stream.store_string(body)
	stream.close()
	return true


func _locale_changed(snapshot: Dictionary) -> void:
	_check(bool(snapshot["busy"]), "Use the existing staged locale callback contract")


func _set_explicit_genesis(simulation: BgrSimulation) -> void:
	var token: BgrSettingsDraft = simulation.settings_open()
	var controls: Dictionary = token.snapshot()
	var texts: PackedStringArray = controls["texts"]
	var keys: PackedStringArray = controls["keys"]
	for item: Array in [["WORLD_WIDTH", "1000"], ["WORLD_HEIGHT", "800"], ["INITIAL_ORGANISMS", "0"], ["INITIAL_CO2", "234567"]]:
		var index: int = keys.find(item[0])
		if not _check(index >= 0, "The existing settings draft exposes genesis"):
			return
		texts[index] = item[1]
	var replaced: Dictionary = token.replace_controls({"texts": texts, "locale": controls["locale"], "automatic_backup": controls["automatic_backup"], "hardware": controls["hardware"], "disable_framebuffer": controls["disable_framebuffer"], "automatic_backup_mode": controls["automatic_backup_mode"]})
	_check(bool(replaced["ok"]), "Stage explicit future genesis through the existing API")
	_check(bool(simulation.settings_apply(token, _locale_changed)["ok"]), "Apply explicit future genesis")


func _run() -> void:
	var owned: String = OS.get_environment("BGR_TEST_ROOT")
	var mode: String = OS.get_environment("BGR_DEFAULTS_CASE")
	if not _check(owned.is_absolute_path() and OS.get_user_data_dir().begins_with(owned + "/"), "Use a fresh isolated profile"):
		_finish()
		return
	if not _check(OS.get_cmdline_user_args().has("--launcher") and mode in ["fresh", "loaded", "import"], "Run one declared Launcher fixture"):
		_finish()
		return
	var native_path: String = OS.get_user_data_dir().path_join("biogenesis-r.preferences.xml")
	_check(not FileAccess.file_exists(native_path), "Each fixture starts without native preferences")
	DirAccess.make_dir_recursive_absolute(OS.get_user_data_dir())
	var custom: String = '<map MAP_XML_VERSION="1.0"><entry key="VERSION" value="700"/><entry key="WORLD_WIDTH" value="1234"/><entry key="WORLD_HEIGHT" value="999"/><entry key="INITIAL_ORGANISMS" value="0"/><entry key="INITIAL_CO2" value="500000"/></map>'
	if mode == "loaded" and not _write(native_path, custom):
		_finish()
		return
	_launcher = (load("res://scenes/launcher.tscn") as PackedScene).instantiate()
	root.add_child(_launcher)
	await process_frame
	var simulation: BgrSimulation = _launcher.get("_simulation") as BgrSimulation
	_check(not is_instance_valid(_launcher.get("_main")), "Launcher-only entry creates no World")
	var before_values: Dictionary = simulation.settings_snapshot()["values"].duplicate(true)
	if mode == "fresh":
		_check(simulation.settings_snapshot()["genesis"] == PackedInt64Array([7000, 7000, 12500]), "Frontend defaults preserve the Core legacy contract before startup")
	elif mode == "import":
		var source: String = owned.path_join("import-before-first-start.xml")
		if not _write(source, custom):
			_finish()
			return
		_check(bool(simulation.preferences_import(source)["ok"]), "Import explicit preferences before the first Start")
		_check(FileAccess.get_file_as_string(source) == custom, "Import preserves source preference bytes")
	var native_before: PackedByteArray = FileAccess.get_file_as_bytes(native_path) if FileAccess.file_exists(native_path) else PackedByteArray()
	_launcher.call("_start_simulation")
	var main: Control = _launcher.get("_main") as Control
	if not _check(is_instance_valid(main), "The actual Launcher opens Main"):
		_finish()
		return
	main.set_process(false)
	var state: Dictionary = simulation.snapshot()
	if mode == "fresh":
		_check(int(state["width"]) == 3500 and int(state["height"]) == 3500 and int(state["initial_attempts"]) == 3125, "Fresh Main uses quarter-area genesis defaults")
		_check(int(state["tick"]) == 0 and int(state["population"]) > 0 and int(state["population"]) <= 3125, "Observe actual accepted population before any tick")
		_check(not FileAccess.file_exists(native_path), "Selecting frontend defaults does not write native preferences")
		_check(int(simulation.settings_snapshot()["values"]["INITIAL_CO2"]) == 125000 and float(state["carbon_dioxide"]) == 0.0 and float(state["oxygen"]) == 125000.0 and float(state["methane"]) == 0.0, "Fresh compact genesis applies125000 CO2 and preserves the3125x40 gas debit")
		var expected_values: Dictionary = before_values.duplicate(true)
		expected_values["WORLD_WIDTH"] = 3500
		expected_values["WORLD_HEIGHT"] = 3500
		expected_values["INITIAL_ORGANISMS"] = 3125
		expected_values["INITIAL_CO2"] = 125000.0
		var actual_values: Dictionary = simulation.settings_snapshot()["values"].duplicate(true)
		for geometry: String in ["WINDOW_X", "WINDOW_Y", "WINDOW_WIDTH", "WINDOW_HEIGHT", "WINDOW_STATE"]:
			expected_values.erase(geometry)
			actual_values.erase(geometry)
		var compared_keys: Array = expected_values.keys()
		for key: String in actual_values:
			if not compared_keys.has(key):
				compared_keys.append(key)
		compared_keys.sort()
		for key: String in compared_keys:
			if not actual_values.has(key) or not expected_values.has(key) or typeof(actual_values.get(key)) != typeof(expected_values.get(key)) or actual_values.get(key) != expected_values.get(key):
				_settings_diff.append({"key": key, "actual": actual_values.get(key), "expected": expected_values.get(key), "actual_type": type_string(typeof(actual_values.get(key))), "expected_type": type_string(typeof(expected_values.get(key)))})
		_co2_types = {"actual": actual_values["INITIAL_CO2"], "expected": expected_values["INITIAL_CO2"], "actual_type": type_string(typeof(actual_values["INITIAL_CO2"])), "expected_type": type_string(typeof(expected_values["INITIAL_CO2"])), "original_integer_literal_type": type_string(typeof(125000))}
		_check(actual_values == expected_values and int(actual_values["INITIAL_ENERGY"]) == 40, "Compact startup preserves every other numeric, color, rate and preference value")
		var defaults: BgrSettingsDraft = simulation.settings_open()
		var legacy: Dictionary = defaults.defaults()
		var legacy_keys: PackedStringArray = legacy["keys"]
		var legacy_texts: PackedStringArray = legacy["texts"]
		_check(float(legacy_texts[legacy_keys.find("INITIAL_CO2")]) == 500000.0 and legacy_texts[legacy_keys.find("WORLD_WIDTH")] == "7000" and legacy_texts[legacy_keys.find("WORLD_HEIGHT")] == "7000" and legacy_texts[legacy_keys.find("INITIAL_ORGANISMS")] == "12500", "Default Values remains the coherent legacy ColorMod preset")
		defaults.cancel()
		_check(int(simulation.settings_snapshot()["values"]["INITIAL_CO2"]) == 125000 and float(simulation.snapshot()["carbon_dioxide"]) == 0.0, "Cancelling legacy defaults leaves the compact world and pending parameters intact")
		_set_explicit_genesis(simulation)
	else:
		_check(int(state["width"]) == 1234 and int(state["height"]) == 999 and int(state["initial_attempts"]) == 0 and int(state["population"]) == 0, "Loaded or imported explicit genesis remains authoritative")
		_check(FileAccess.get_file_as_bytes(native_path) == native_before, "Default selection does not rewrite loaded preference bytes")
		_check(int(simulation.settings_snapshot()["values"]["INITIAL_CO2"]) == 500000 and float(state["carbon_dioxide"]) == 500000.0, "Loaded or imported500000 CO2 remains authoritative")
	await _launcher.call("_stop_simulation")
	if mode == "fresh":
		_launcher.call("_start_simulation")
		main = _launcher.get("_main") as Control
		if _check(is_instance_valid(main), "Reopen Main after an explicit genesis change"):
			main.set_process(false)
			state = simulation.snapshot()
			_check(int(state["width"]) == 1000 and int(state["height"]) == 800 and int(state["initial_attempts"]) == 0 and int(state["population"]) == 0, "Reopening retains explicit genesis instead of reapplying fresh defaults")
			_check(int(simulation.settings_snapshot()["values"]["INITIAL_CO2"]) == 234567 and float(state["carbon_dioxide"]) == 234567.0, "Reopening preserves explicit CO2 rather than reapplying fresh defaults")
			await _launcher.call("_stop_simulation")
	_launcher.queue_free()
	await process_frame
	_finish()


func _finish() -> void:
	print("BGR_NEW_WORLD_DEFAULTS " + JSON.stringify({"case": OS.get_environment("BGR_DEFAULTS_CASE"), "checks": _checks, "failures": _failures, "settings_diff": _settings_diff, "co2_types": _co2_types, "result": "PASS" if _failures.is_empty() else "FAIL"}))
	quit(0 if _failures.is_empty() else 1)
