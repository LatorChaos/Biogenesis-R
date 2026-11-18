extends RefCounted

const PATH: String = "user://display.cfg"
const MAX_BYTES: int = 16 * 1024
const DEFAULTS: Dictionary = {
	"fullscreen": true,
	"cinema_enabled": true,
	"continuous_display": true,
	"idle_seconds": 5.0,
	"camera_hold_seconds": 20.0,
	"camera_motion_percent": 12.5,
	"camera_settled_hold_seconds": 200.0,
	"slide_interval_seconds": 60.0,
	"slide_dwell_seconds": 8.0
}
const RANGES: Dictionary = {
	"idle_seconds": Vector2(1.0, 60.0),
	"camera_hold_seconds": Vector2(5.0, 300.0),
	"camera_motion_percent": Vector2(1.0, 100.0),
	"camera_settled_hold_seconds": Vector2(5.0, 3600.0),
	"slide_interval_seconds": Vector2(10.0, 3600.0),
	"slide_dwell_seconds": Vector2(2.0, 60.0)
}

var values: Dictionary = DEFAULTS.duplicate()


func load_settings() -> Dictionary:
	values = DEFAULTS.duplicate()
	var loaded: Dictionary = _read_config()
	if not loaded["ok"]:
		return _outcome(str(loaded["error"]))
	var config: ConfigFile = loaded["config"]
	var candidate: Dictionary = DEFAULTS.duplicate()
	for key: String in DEFAULTS:
		if config.has_section_key("display", key):
			candidate[key] = config.get_value("display", key)
	var error: String = _validate(candidate)
	if not error.is_empty():
		return _outcome(error)
	values = candidate
	return _outcome("")


func save_settings() -> Dictionary:
	var error: String = _validate(values)
	if not error.is_empty():
		return _outcome(error)
	var loaded: Dictionary = _read_config()
	if not loaded["ok"]:
		return _outcome(str(loaded["error"]))
	var config: ConfigFile = loaded["config"]
	for key: String in DEFAULTS:
		config.set_value("display", key, values[key])
	var body: PackedByteArray = config.encode_to_text().to_utf8_buffer()
	if body.size() > MAX_BYTES:
		return _outcome("Display preferences exceed 16 KiB")
	var temporary: String = PATH + ".tmp-" + str(OS.get_process_id()) + "-" + str(Time.get_ticks_usec())
	if FileAccess.file_exists(temporary) or DirAccess.dir_exists_absolute(temporary):
		return _outcome("Display preference temporary path already exists")
	var file: FileAccess = FileAccess.open(temporary, FileAccess.WRITE)
	if file == null:
		return _outcome("Cannot create display preferences: " + error_string(FileAccess.get_open_error()))
	file.store_buffer(body)
	file.flush()
	var write_error: Error = file.get_error()
	file.close()
	if write_error != OK:
		return _discard(temporary, "Cannot write display preferences: " + error_string(write_error))
	file = FileAccess.open(temporary, FileAccess.READ)
	if file == null:
		return _discard(temporary, "Cannot verify display preferences")
	var actual: PackedByteArray = file.get_buffer(MAX_BYTES + 1)
	file.close()
	if actual != body:
		return _discard(temporary, "Display preference write verification failed")
	var renamed: Error = _replace_file(temporary, PATH)
	if renamed != OK:
		return _discard(temporary, "Cannot replace display preferences: " + error_string(renamed))
	return _outcome("")


func _validate(candidate: Dictionary) -> String:
	if candidate.size() != DEFAULTS.size():
		return "Unexpected display preference fields"
	for key: String in DEFAULTS:
		if not candidate.has(key):
			return "Missing display preference: " + key
		var value: Variant = candidate[key]
		if key in ["fullscreen", "cinema_enabled", "continuous_display"]:
			if not value is bool:
				return "Display preference must be boolean: " + key
		else:
			if not (value is int or value is float) or not is_finite(float(value)):
				return "Display preference must be a finite number: " + key
			var limits: Vector2 = RANGES[key]
			if float(value) < limits.x or float(value) > limits.y:
				return "Display preference is outside its range: " + key
	if float(candidate["slide_dwell_seconds"]) > float(candidate["slide_interval_seconds"]):
		return "Slide dwell cannot exceed its interval"
	return ""


func _read_config() -> Dictionary:
	var config: ConfigFile = ConfigFile.new()
	if not FileAccess.file_exists(PATH):
		if DirAccess.dir_exists_absolute(PATH):
			return _outcome("Display preference path is a directory")
		return {"ok": true, "error": "", "config": config}
	var file: FileAccess = FileAccess.open(PATH, FileAccess.READ)
	if file == null:
		return _outcome("Cannot read display preferences: " + error_string(FileAccess.get_open_error()))
	var length: int = file.get_length()
	if length > MAX_BYTES:
		file.close()
		return _outcome("Display preferences exceed 16 KiB")
	var bytes: PackedByteArray = file.get_buffer(MAX_BYTES + 1)
	var stable: bool = bytes.size() == length and file.get_length() == length
	var read_error: Error = file.get_error()
	file.close()
	if not stable or bytes.size() > MAX_BYTES or read_error not in [OK, ERR_FILE_EOF]:
		return _outcome("Display preference read failed or changed")
	var text: String = bytes.get_string_from_utf8()
	if text.to_utf8_buffer() != bytes or not _scalar_text_only(text):
		return _outcome("Display preferences require UTF-8 scalar values")
	var parsed: Error = config.parse(text)
	if parsed != OK:
		return _outcome("Malformed display preferences: " + error_string(parsed))
	for section: String in config.get_sections():
		for key: String in config.get_section_keys(section):
			var value: Variant = config.get_value(section, key)
			if not (value is bool or value is int or value is String or (value is float and is_finite(value))):
				return _outcome("Unsupported display preference value")
	return {"ok": true, "error": "", "config": config}


func _scalar_text_only(text: String) -> bool:
	# ConfigFile can instantiate Variant constructors; this UI file stores scalars only
	for line: String in text.split("\n"):
		var stripped: String = line.strip_edges()
		if stripped.begins_with("["):
			var end: int = stripped.find("]")
			if end <= 1:
				return false
			var suffix: String = stripped.substr(end + 1).strip_edges()
			if not suffix.is_empty() and not suffix.begins_with(";"):
				return false
			var section: String = stripped.substr(1, end - 1)
			for index: int in range(section.length()):
				if "(){}[]".contains(section[index]):
					return false
			continue
		var quoted: bool = false
		var escaped: bool = false
		for index: int in range(line.length()):
			var character: String = line[index]
			if quoted:
				if escaped:
					escaped = false
				elif character == "\\":
					escaped = true
				elif character == "\"":
					quoted = false
			elif character == ";":
				break
			elif character == "\"":
				quoted = true
			elif "(){}[]".contains(character):
				return false
		if quoted:
			return false
	return true


func _replace_file(source: String, destination: String) -> Error:
	return DirAccess.rename_absolute(ProjectSettings.globalize_path(source), ProjectSettings.globalize_path(destination))


func _discard(path: String, error: String) -> Dictionary:
	var removed: Error = DirAccess.remove_absolute(ProjectSettings.globalize_path(path))
	return _outcome(error if removed == OK else error + "; temporary cleanup failed: " + error_string(removed))


func _outcome(error: String) -> Dictionary:
	return {"ok": error.is_empty(), "error": error}
