extends SceneTree

const Preferences = preload("res://ui/display_preferences.gd")

class FailedRename extends Preferences:
	func _replace_file(_source: String, _destination: String) -> Error:
		return ERR_CANT_CREATE

var _checks: int = 0
var _failures: Array[String] = []


func _initialize() -> void:
	_run.call_deferred()


func _check(condition: bool, description: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(description)


func _write(text: String) -> void:
	var file: FileAccess = FileAccess.open(Preferences.PATH, FileAccess.WRITE)
	assert(file != null, "Create only the isolated test configuration")
	file.store_string(text)
	file.close()


func _run() -> void:
	if FileAccess.file_exists(Preferences.PATH) or DirAccess.dir_exists_absolute(Preferences.PATH):
		_check(false, "Require an isolated profile without existing display preferences")
		_finish()
		return
	var settings: Preferences = Preferences.new()
	_check(settings.load_settings()["ok"] and settings.values == Preferences.DEFAULTS, "Missing file uses exact display defaults")
	_check(not FileAccess.file_exists(Preferences.PATH), "Loading defaults does not create a configuration")
	_write("[display]\nfullscreen=false\nfuture_toggle=true\n\n[future]\nmessage=\"keep Object(quoted) [text]\"\n")
	_check(settings.load_settings()["ok"] and settings.values["fullscreen"] == false, "Read typed values and default missing fields")
	_check(settings.values["continuous_display"] == true, "Existing configurations default to continuous display")
	settings.values["continuous_display"] = false
	settings.values["cinema_enabled"] = false
	settings.values["idle_seconds"] = 7.0
	settings.values["camera_hold_seconds"] = 30.0
	settings.values["slide_interval_seconds"] = 90.0
	settings.values["slide_dwell_seconds"] = 10.0
	var expected: Dictionary = settings.values.duplicate()
	_check(settings.save_settings()["ok"], "Save a validated display configuration")
	var reread: Preferences = Preferences.new()
	_check(reread.load_settings()["ok"] and reread.values == expected, "Roundtrip all display values including disabled continuous display")
	var stored: ConfigFile = ConfigFile.new()
	_check(stored.load(Preferences.PATH) == OK and stored.get_value("future", "message") == "keep Object(quoted) [text]"
		and stored.get_value("display", "future_toggle") == true, "Preserve safe unknown keys and sections")
	var original: PackedByteArray = FileAccess.get_file_as_bytes(Preferences.PATH)
	for pair: Array in [["fullscreen", 1], ["continuous_display", 1], ["continuous_display", "true"], ["idle_seconds", "5"], ["idle_seconds", NAN],
		["camera_hold_seconds", INF], ["idle_seconds", 0.0], ["slide_dwell_seconds", 61.0]]:
		settings.values = expected.duplicate()
		settings.values[str(pair[0])] = pair[1]
		_check(not settings.save_settings()["ok"] and FileAccess.get_file_as_bytes(Preferences.PATH) == original,
			"Invalid save preserves existing bytes: " + str(pair[0]))
	settings.values = expected.duplicate()
	settings.values["slide_interval_seconds"] = 10.0
	settings.values["slide_dwell_seconds"] = 11.0
	_check(not settings.save_settings()["ok"] and FileAccess.get_file_as_bytes(Preferences.PATH) == original,
		"Reject dwell longer than its interval without changing the file")
	var failed: FailedRename = FailedRename.new()
	failed.values = expected.duplicate()
	failed.values["fullscreen"] = true
	_check(not failed.save_settings()["ok"] and FileAccess.get_file_as_bytes(Preferences.PATH) == original,
		"A replacement failure preserves the complete previous configuration")
	var directory: DirAccess = DirAccess.open("user://")
	var leftovers: PackedStringArray = directory.get_files()
	_check(not Array(leftovers).any(func(name: String) -> bool: return name.begins_with("display.cfg.tmp-")),
		"Failed replacement cleans only its temporary file")
	for malformed: String in ["[display]\ncontinuous_display=1\n", "[display]\ncontinuous_display=\"false\"\n", "[display]\nidle_seconds=\"fast\"\n", "[display]\nidle_seconds=[1]\n",
		"[future]\nvalue=Object(RefCounted,)\n", " ".repeat(Preferences.MAX_BYTES + 1)]:
		_write(malformed)
		_check(not reread.load_settings()["ok"] and reread.values == Preferences.DEFAULTS,
			"Malformed or oversized input falls back to exact defaults")
	DirAccess.remove_absolute(ProjectSettings.globalize_path(Preferences.PATH))
	_finish()


func _finish() -> void:
	print(JSON.stringify({"status": "PASS" if _failures.is_empty() else "FAIL", "checks": _checks, "failures": _failures}))
	quit(0 if _failures.is_empty() else 1)
