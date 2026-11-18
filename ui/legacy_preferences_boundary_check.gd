extends SceneTree

const SOURCE: String = '<map MAP_XML_VERSION="1.0"><entry key="VERSION" value="700"/><entry key="WORLD_WIDTH" value="1200"/><entry key="WORLD_HEIGHT" value="800"/><entry key="INITIAL_ORGANISMS" value="0"/><entry key="LOCALE" value="PT_BR"/><entry key="DELAY" value="-17"/><entry key="MAX_VEL" value="0.125"/><entry key="ORGANISMS_VECTOR_SIZE" value="13"/><entry key="AUTO_BACKUP" value="true"/><entry key="HARDWARE_ACCELERATION" value="1"/><entry key="WINDOW_X" value="22"/><entry key="WINDOW_Y" value="33"/><entry key="WINDOW_WIDTH" value="1000"/><entry key="WINDOW_HEIGHT" value="760"/><entry key="WINDOW_STATE" value="6"/><entry key="BGR_CRISP_LINES" value="false"/><entry key="BGR_AUTOMATIC_BACKUP_MODE" value="invalid-native-policy"/></map>'

var _checks: int = 0
var _failures: Array[String] = []
var _simulation: BgrSimulation
var _source: String
var _native: String
var _callbacks: int = 0


func _initialize() -> void:
	_run.call_deferred()


func _check(value: bool, message: String) -> void:
	_checks += 1
	if not value:
		_failures.append(message)


func _write(path: String, text: String) -> void:
	var file: FileAccess = FileAccess.open(path, FileAccess.WRITE)
	_check(file != null, "Cannot create owned preference fixture")
	if file != null:
		file.store_string(text)
		file.close()


func _reject(path: String) -> void:
	var preferences: Dictionary = _simulation.preferences_snapshot()
	var state: Dictionary = _simulation.snapshot()
	var native_bytes: PackedByteArray = FileAccess.get_file_as_bytes(_native)
	var source_bytes: PackedByteArray = FileAccess.get_file_as_bytes(path) if FileAccess.file_exists(path) else PackedByteArray()
	var result: Dictionary = _simulation.preferences_import(path)
	_check(not bool(result["ok"]) and not bool(result["applied"]) and not bool(result["saved"]) and not bool(result["replacement_committed"]), "Rejected import reported application or a save")
	_check(_simulation.preferences_snapshot() == preferences and _simulation.snapshot() == state, "Rejected import changed its native status, settings or session")
	_check(FileAccess.get_file_as_bytes(_native) == native_bytes and (not FileAccess.file_exists(path) or FileAccess.get_file_as_bytes(path) == source_bytes), "Rejected import changed source or native bytes")


func _during_apply(_payload: Dictionary) -> void:
	_callbacks += 1
	var preferences: Dictionary = _simulation.preferences_snapshot()
	var bytes: PackedByteArray = FileAccess.get_file_as_bytes(_native)
	var result: Dictionary = _simulation.preferences_import(_source)
	_check(not bool(result["ok"]) and not bool(result["applied"]) and str(result["error"]).contains("busy"), "Callback import did not reject before parsing or reading the absent settings owner")
	_check(_simulation.preferences_snapshot() == preferences and FileAccess.get_file_as_bytes(_native) == bytes, "Callback import poisoned native status or changed bytes")


func _run() -> void:
	var owned: String = OS.get_environment("BGR_TEST_ROOT")
	var directory: String = OS.get_user_data_dir()
	if not owned.is_absolute_path() or not directory.begins_with(owned + "/"):
		push_error("An isolated owned preference profile is required")
		quit(2)
		return
	_native = directory.path_join("biogenesis-r.preferences.xml")
	_source = directory.path_join("legacy import source.xml")
	if FileAccess.file_exists(_native) or FileAccess.file_exists(_source):
		push_error("Preserve prior preference fixtures")
		quit(2)
		return
	_write(_source, SOURCE)
	_simulation = BgrSimulation.new()
	var uninitialized: Dictionary = _simulation.preferences_import(_source)
	_check(not bool(uninitialized["ok"]) and not bool(uninitialized["applied"]) and not FileAccess.file_exists(_native), "Uninitialized import touched native preferences")
	_write(_native, '<map><entry key="VERSION" value="700"/><entry key="BGR_CRISP_LINES" value="true"/><entry key="BGR_AUTOMATIC_BACKUP_MODE" value="managed"/><entry key="unknown" value="keep"/></map>')
	var initial: Dictionary = _simulation.preferences_initialize()
	_check(bool(initial["ok"]), "Owned native fixture did not initialize")
	var defaults: Dictionary = initial["settings"]
	var imported: Dictionary = _simulation.preferences_import(_source)
	_check(bool(imported["ok"]) and bool(imported["applied"]) and bool(imported["saved"]) and bool(imported["replacement_committed"]), "Valid import did not commit and apply")
	var settings: Dictionary = _simulation.settings_snapshot()
	_check(settings["genesis"] == PackedInt64Array([1200, 800, 0]), "Legacy genesis values differ")
	_check(str(settings["language"]) == "pt_br" and int(settings["locale"]) == 1, "Legacy locale normalization or fallback index differs")
	_check(int(settings["values"]["DELAY"]) == -17 and float(settings["values"]["MAX_VEL"]) == 0.125 and int(settings["values"]["ORGANISMS_VECTOR_SIZE"]) == 13, "Raw or hidden Source preferences were filtered through dialog restrictions")
	_check(bool(settings["values"]["AUTO_BACKUP"]) and bool(settings["crisp_lines"]) and int(settings["automatic_backup_mode"]) == 1, "Source backup boolean or native-only choices changed")
	_check(imported["hardware"] == {"requested": 1, "effective": 2, "persisted_override": 0}, "Source graphics startup metadata differs")
	_check(_simulation.preferences_snapshot()["geometry"] == PackedInt32Array([22, 33, 1000, 760, 6]), "Imported geometry differs")
	_check(FileAccess.get_file_as_bytes(_source) == SOURCE.to_utf8_buffer(), "Successful import changed source bytes")
	_check(FileAccess.get_file_as_string(_native).contains('key="unknown" value="keep"'), "Import discarded unknown native keys")
	var restarted: BgrSimulation = BgrSimulation.new()
	var loaded: Dictionary = restarted.preferences_initialize()
	_check(bool(loaded["ok"]) and loaded["settings"] == settings and loaded["geometry"] == PackedInt32Array([22, 33, 1000, 760, 6]), "A fresh preference owner did not recover the imported settings")
	_write(_source, '<map><entry key="VERSION" value="700"/><entry key="MAX_VEL" value="invalid"/></map>')
	_check(bool(_simulation.preferences_import(_source)["ok"]), "Partial Source map was rejected")
	var partial: Dictionary = _simulation.settings_snapshot()
	_check(partial["genesis"] == defaults["genesis"] and partial["values"]["MAX_VEL"] == defaults["values"]["MAX_VEL"] and partial["language"] == defaults["language"], "Partial import merged old Source values or used the previous imported locale as its default")
	_check(bool(partial["crisp_lines"]) and int(partial["automatic_backup_mode"]) == 1, "Partial import lost native-only choices")
	for text: String in ["broken", '<map/>', '<map><entry key="VERSION" value="701"/></map>', '<preferences EXTERNAL_XML_VERSION="1.0"><root type="user"/></preferences>']:
		_write(_source, text)
		_reject(_source)
	_reject(_native)
	_reject(directory)
	_reject(directory.path_join("missing.xml"))
	_reject("relative.xml")
	var token: BgrSettingsDraft = _simulation.settings_open()
	var controls: Dictionary = token.snapshot()
	_check(bool(token.replace_controls({"texts": controls["texts"], "locale": (int(controls["locale"]) + 1) % 6, "automatic_backup": controls["automatic_backup"], "hardware": controls["hardware"], "disable_framebuffer": controls["disable_framebuffer"], "automatic_backup_mode": controls["automatic_backup_mode"]})["ok"]), "Cannot prepare real settings callback")
	_check(bool(_simulation.settings_apply(token, _during_apply)["ok"]) and _callbacks == 1, "Synchronous import rejection callback did not execute")
	_write(_source, SOURCE)
	_check(bool(_simulation.start_session(827364, 1200, 800, 0)["ok"]), "Cannot start the running-rejection fixture")
	_reject(_source)
	_check(bool(_simulation.release_world()["ok"]), "Cannot stop the fixture")
	var stopped: Dictionary = _simulation.snapshot()
	_check(not bool(stopped["world_ready"]) and bool(stopped["session_ready"]), "Stop did not retain the detached session")
	_write(_source, SOURCE)
	_check(bool(_simulation.preferences_import(_source)["ok"]), "Import after Start/Stop failed")
	var after: Dictionary = _simulation.snapshot()
	_check(not bool(after["world_ready"]) and bool(after["session_ready"]) and after["seed"] == stopped["seed"], "Stopped import replaced the session or seed")
	_check(FileAccess.get_file_as_bytes(_source) == SOURCE.to_utf8_buffer(), "Stopped import changed source bytes")
	_write(_native, '<map><entry key="VERSION" value="701"/><entry key="unknown" value="future"/></map>')
	_reject(_source)
	_write(_native, "broken native preferences")
	_reject(_source)
	print("BGR_LEGACY_PREFERENCES_BOUNDARY_RESULT " + JSON.stringify({"status": "PASS" if _failures.is_empty() else "FAIL", "checks": _checks, "failures": _failures, "real_Java_profiles": 0}))
	quit(0 if _failures.is_empty() else 1)
