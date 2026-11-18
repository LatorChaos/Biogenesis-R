extends SceneTree

var _checks: int = 0
var _path: String = ""
var _simulation: BgrSimulation
var _token: BgrSettingsDraft
var _events: int = 0
var _callback_file_before: PackedByteArray


func _check(value: bool, message: String) -> void:
	_checks += 1
	if not value:
		printerr("BGR_PREFERENCES_BOUNDARY_FAIL " + message)
		quit(1)
		assert(false, message)


func _write(text: String) -> void:
	var file: FileAccess = FileAccess.open(_path, FileAccess.WRITE)
	_check(file != null, "Cannot create owned preference fixture")
	file.store_string(text)
	file.close()


func _locale_changed(payload: Dictionary) -> void:
	_events += 1
	_check(str(payload["language"]) == "de" and int(payload["values"]["WORLD_WIDTH"]) == 1000 and int(payload["values"]["DELAY"]) == 0, "Locale callback did not precede numeric application")
	var preferences: Dictionary = _simulation.preferences_snapshot()
	_check(preferences["settings"] == payload and bool(preferences["settings"]["busy"]), "Preference read did not reuse the staged presentation")
	_check(preferences["geometry"] == PackedInt32Array([22, 33, 900, 700, 0]) and bool(preferences["settings"]["crisp_lines"]), "Callback lost nondefault geometry or Crisp")
	_check(_simulation.preferences_initialize() == preferences, "Repeated initialization did not return the same staged read")
	preferences["settings"]["values"]["WINDOW_X"] = -900
	preferences["geometry"][0] = -900
	var reread: Dictionary = _simulation.preferences_snapshot()
	_check(int(reread["settings"]["values"]["WINDOW_X"]) == 22 and int(reread["geometry"][0]) == 22, "Copied preference payload changed the staged owner")
	var save: Dictionary = _simulation.preferences_save(PackedInt32Array([1, 2, 1000, 800, 0]))
	_check(not bool(save["ok"]) and not bool(save["saved"]) and "busy" in str(save["error"]), "Reentrant preference save was not BUSY")
	var crisp: Dictionary = _simulation.set_crisp_lines(false)
	_check(not bool(crisp["ok"]) and "busy" in str(crisp["error"]), "Reentrant Crisp write was not BUSY")
	_check(not bool(_token.defaults()["ok"]), "Reentrant draft write was accepted")
	var nested: Dictionary = _simulation.settings_apply(_token, _locale_changed)
	_check(not bool(nested["ok"]) and "reentrant" in str(nested["error"]), "Reentrant settings application was accepted")
	_check(FileAccess.get_file_as_bytes(_path) == _callback_file_before, "Callback reentry changed the native file")


func _initialize() -> void:
	var owned: String = OS.get_environment("BGR_TEST_ROOT")
	if not owned.is_absolute_path() or not OS.get_user_data_dir().begins_with(owned + "/"):
		printerr("BGR_PREFERENCES_BOUNDARY_FAIL Owned profile is required")
		quit(1)
		return
	_path = OS.get_user_data_dir().path_join("biogenesis-r.preferences.xml")
	_check(not FileAccess.file_exists(_path), "Boundary needs a fresh owned profile")
	var simulation: BgrSimulation = BgrSimulation.new()
	_check(not FileAccess.file_exists(_path), "Pure constructor wrote preferences")
	_check(not bool(simulation.snapshot()["world_ready"]), "Constructor created a World")
	var initialized: Dictionary = simulation.preferences_initialize()
	_check(bool(initialized["ok"]) and str(initialized["status"]) == "missing", "Missing file did not use defaults")
	_check(not bool(initialized["world_ready"]) and not bool(initialized["session_ready"]), "Initialization created a session")
	_check(not FileAccess.file_exists(_path), "Initialization wrote missing preferences")
	var defaults: Dictionary = initialized["settings"]
	_check(defaults["genesis"] == PackedInt64Array([7000, 7000, 12500]), "Source defaults changed")
	_check(not bool(defaults["crisp_lines"]), "Smooth is not the default")
	var saved: Dictionary = simulation.preferences_save(PackedInt32Array([20, 30, 1000, 760, 0]))
	_check(bool(saved["ok"]) and bool(saved["saved"]), "Native save did not succeed")
	var bytes_before: PackedByteArray = FileAccess.get_file_as_bytes(_path)
	_check(not FileAccess.get_file_as_string(_path).contains('key="SEED"'), "Save persisted a seed")
	var invalid: Dictionary = simulation.preferences_save(PackedInt32Array([1, 2]))
	_check(not bool(invalid["ok"]) and not bool(invalid["saved"]), "Malformed geometry reported success")
	_check(FileAccess.get_file_as_bytes(_path) == bytes_before, "Invalid geometry changed the file")
	_write('<map MAP_XML_VERSION="1.0"><entry key="VERSION" value="700"/><entry key="DELAY" value="0"/><entry key="LOCALE" value="PT_BR"/><entry key="WORLD_WIDTH" value="1000"/><entry key="WORLD_HEIGHT" value="900"/><entry key="INITIAL_ORGANISMS" value="0"/><entry key="BGR_CRISP_LINES" value="true"/><entry key="unknown" value="keep"/></map>')
	_check(simulation.preferences_initialize()["settings"]["genesis"] == defaults["genesis"], "Repeated initialization reread the file")
	var restarted: BgrSimulation = BgrSimulation.new()
	var loaded: Dictionary = restarted.preferences_initialize()
	_check(str(loaded["status"]) == "loaded", "Native map did not load")
	_check(str(loaded["settings"]["language"]) == "pt_br" and int(loaded["settings"]["locale"]) == 1, "Unsupported locale identity/index changed")
	_check(int(loaded["settings"]["values"]["DELAY"]) == 0, "Stored zero delay was repaired")
	_check(bool(loaded["settings"]["crisp_lines"]), "Crisp extension did not load")
	var configured: Dictionary = restarted.configure(827365, 1000, 900, 0)
	_check(bool(configured["ok"]) and str(configured["seed"]) == "827365", "Preferences replaced the explicit seed")
	_check(bool(restarted.set_crisp_lines(false)["ok"]), "Typed display choice failed")
	_check(bool(restarted.preferences_save(PackedInt32Array([22, 33, 900, 700, 0]))["ok"]), "Loaded native save failed")
	var text: String = FileAccess.get_file_as_string(_path)
	_check(text.contains('key="unknown" value="keep"'), "Unknown key disappeared")
	_check(text.contains('key="LOCALE" value="pt_br"'), "Unsupported locale was replaced by a fallback")
	_check(not text.contains('key="SEED"'), "Restart/save persisted a seed")
	_simulation = BgrSimulation.new()
	_check(bool(_simulation.preferences_initialize()["ok"]), "Callback fixture did not load")
	_check(bool(_simulation.set_crisp_lines(true)["ok"]), "Callback Crisp fixture failed")
	_check(bool(_simulation.preferences_save(PackedInt32Array([22, 33, 900, 700, 0]))["ok"]), "Callback geometry fixture failed")
	_token = _simulation.settings_open()
	var controls: Dictionary = _token.snapshot()
	var keys: PackedStringArray = controls["keys"]
	var texts: PackedStringArray = controls["texts"]
	var width_index: int = keys.find("WORLD_WIDTH")
	_check(width_index >= 0, "Callback width editor is absent")
	texts[width_index] = "1234"
	_check(bool(_token.replace_controls({"texts": texts, "locale": 3, "automatic_backup": controls["automatic_backup"], "hardware": controls["hardware"], "disable_framebuffer": controls["disable_framebuffer"]})["ok"]), "Callback controls were rejected")
	_callback_file_before = FileAccess.get_file_as_bytes(_path)
	var applied: Dictionary = _simulation.settings_apply(_token, _locale_changed)
	_check(bool(applied["ok"]) and _events == 1, "Real synchronous locale callback failed")
	_check(str(applied["language"]) == "de" and int(applied["values"]["WORLD_WIDTH"]) == 1234 and bool(applied["crisp_lines"]), "Apply did not restore the single settings owner")
	_check(_simulation.preferences_snapshot()["geometry"] == PackedInt32Array([22, 33, 900, 700, 0]), "Apply changed callback geometry")
	_check(FileAccess.get_file_as_bytes(_path) == _callback_file_before, "Settings apply or rejected writes changed preference bytes")
	_write('<map MAP_XML_VERSION="1.0"><entry key="VERSION" value="701"/><entry key="unknown" value="future"/></map>')
	var future_bytes: PackedByteArray = FileAccess.get_file_as_bytes(_path)
	var future: BgrSimulation = BgrSimulation.new()
	_check(not bool(future.preferences_initialize()["ok"]), "Future version did not report an error")
	_check(not bool(future.preferences_save(PackedInt32Array([0, 0, 900, 700, 0]))["ok"]), "Future version save reported success")
	_check(FileAccess.get_file_as_bytes(_path) == future_bytes, "Future version bytes changed")
	_write("broken native preferences")
	var corrupt: BgrSimulation = BgrSimulation.new()
	_check(not bool(corrupt.preferences_initialize()["ok"]), "Corrupt file did not report an error")
	_check(not bool(corrupt.preferences_save(PackedInt32Array([0, 0, 900, 700, 0]))["ok"]), "Corrupt file save reported success")
	_check(FileAccess.get_file_as_string(_path) == "broken native preferences", "Corrupt bytes changed")
	print("BGR_PREFERENCES_BOUNDARY_PASS " + str(_checks))
	quit(0)
