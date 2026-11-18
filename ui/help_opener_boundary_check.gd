extends SceneTree

const HelpAboutScript = preload("res://ui/help_about.gd")
const MainScript = preload("res://ui/main.gd")
const RUN_PERMISSIONS: int = FileAccess.UNIX_READ_OWNER | FileAccess.UNIX_WRITE_OWNER | FileAccess.UNIX_EXECUTE_OWNER | FileAccess.UNIX_READ_GROUP | FileAccess.UNIX_EXECUTE_GROUP | FileAccess.UNIX_READ_OTHER | FileAccess.UNIX_EXECUTE_OTHER
const FILE_PERMISSIONS: int = FileAccess.UNIX_READ_OWNER | FileAccess.UNIX_WRITE_OWNER | FileAccess.UNIX_READ_GROUP | FileAccess.UNIX_READ_OTHER

var _failures: Array[String] = []
var _errors: Array[String] = []
var _checks: int = 0
var _log: String


func _initialize() -> void:
	_run.call_deferred()


func _run() -> void:
	if OS.get_name() != "Linux" or not FileAccess.file_exists("/proc/self/stat"):
		push_error("The opener check requires Linux process observations and owned executable fixtures")
		quit(2)
		return
	var run: String = OS.get_environment("BGR_TEST_ROOT")
	if not run.is_absolute_path() or not DirAccess.dir_exists_absolute(run):
		push_error("Supply an owned absolute BGR_TEST_ROOT")
		quit(2)
		return
	var fixture: String = run.path_join("opener_fixtures")
	if DirAccess.dir_exists_absolute(fixture) or FileAccess.file_exists(fixture):
		push_error("Preserve the prior opener fixture")
		quit(2)
		return
	var body: String = FileAccess.get_file_as_string("res://ui/help_opener_fixture.py")
	var paths: Dictionary = {}
	for name: String in ["missing", "nonexec", "space folder", "gio_only", "legacy", "broken", "delayed", "symlink/parent/xdg-open", "symlink/other/subdir"]:
		var path: String = fixture.path_join(name)
		if DirAccess.make_dir_recursive_absolute(path) != OK:
			quit(2)
			return
		paths[name] = path
	for entry: Array in [["nonexec", "xdg-open", body, FILE_PERMISSIONS], ["space folder", "xdg-open", body, RUN_PERMISSIONS], ["gio_only", "gio", body, RUN_PERMISSIONS], ["legacy", "iceweasel", body, RUN_PERMISSIONS], ["broken", "xdg-open", "#!" + fixture.path_join("missing_interpreter") + "\n", RUN_PERMISSIONS], ["delayed", "xdg-open", body, RUN_PERMISSIONS]]:
		if not _write_fixture(str(paths[entry[0]]).path_join(str(entry[1])), str(entry[2]), int(entry[3])):
			quit(2)
			return
	_check(HelpAboutScript._find_opener(str(paths["nonexec"]) + ":" + str(paths["space folder"]), fixture) == str(paths["space folder"]).path_join("xdg-open"), "Skip non-executable candidate and preserve spaced absolute path")
	_check(HelpAboutScript._find_opener("missing:space folder", fixture) == str(paths["space folder"]).path_join("xdg-open"), "Resolve relative PATH against supplied actual cwd")
	_check(HelpAboutScript._find_opener("", str(paths["space folder"])) == str(paths["space folder"]).path_join("xdg-open"), "Empty PATH segment denotes actual cwd")
	_check(HelpAboutScript._find_opener(str(paths["legacy"]), fixture) == str(paths["legacy"]).path_join("iceweasel"), "Preserve Original browser fallback")
	var real_opener: String = fixture.path_join("symlink/other/xdg-open")
	if not _write_fixture(real_opener, body, RUN_PERMISSIONS):
		quit(2)
		return
	var directory: DirAccess = DirAccess.open(fixture)
	if directory == null or directory.create_link(fixture.path_join("symlink/other/subdir"), fixture.path_join("symlink/parent/link")) != OK:
		quit(2)
		return
	var linked_path: String = fixture.path_join("symlink/parent/link/..")
	_check(HelpAboutScript._find_opener(linked_path, fixture) == linked_path + "/xdg-open", "Preserve symlink parent traversal despite a directory at the lexically collapsed decoy")
	var saved: Dictionary = {}
	for key: String in ["PATH", "BGR_HELP_OPENER_LOG", "BGR_HELP_OPENER_DELAY", "BGR_HELP_OPENER_STATUS"]:
		saved[key] = {"present": OS.has_environment(key), "value": OS.get_environment(key)}
	_log = fixture.path_join("argv.jsonl")
	OS.set_environment("BGR_HELP_OPENER_LOG", _log)
	OS.set_environment("BGR_HELP_OPENER_DELAY", "0")
	OS.set_environment("BGR_HELP_OPENER_STATUS", "0")
	var target: String = fixture.path_join("manual with spaces # ; $(not-a-command).html")
	for name: String in ["missing", "nonexec"]:
		_errors.clear()
		OS.set_environment("PATH", str(paths[name]))
		var pid: int = HelpAboutScript.open_manual(target, self, _capture_error)
		_check(pid < 0 and _errors.size() == 1 and _errors[0].contains("Could not find web browser"), name + ": visible failure without a child")
	for name: String in ["space folder", "gio_only"]:
		_errors.clear()
		OS.set_environment("PATH", str(paths[name]))
		var before: int = _records().size()
		var pid: int = HelpAboutScript.open_manual(target, self, _capture_error)
		var reaped: bool = await _wait_reaped(pid)
		var rows: Array = _records()
		var argv: Array = ["open", target] if name == "gio_only" else [target]
		_check(pid > 0 and reaped and _errors.is_empty() and rows.size() == before + 1 and rows[-1]["pid"] == pid and rows[-1]["argv"] == argv and OS.get_process_exit_code(pid) == 0, name + ": one direct argv spawn and reaped zero exit")
	_errors.clear()
	OS.set_environment("PATH", str(paths["broken"]))
	var broken_pid: int = HelpAboutScript.open_manual(target, self, _capture_error)
	var broken_reaped: bool = await _wait_reaped(broken_pid)
	_check(broken_pid > 0 and broken_reaped and _errors.size() == 1 and _errors[0].contains("exited with status"), "Missing interpreter fails after fork and is reaped")
	_errors.clear()
	OS.set_environment("PATH", str(paths["delayed"]))
	OS.set_environment("BGR_HELP_OPENER_DELAY", "0.5")
	var delayed_pid: int = HelpAboutScript.open_manual(target, self, _capture_error)
	await process_frame
	_check(delayed_pid > 0 and _pid_present(delayed_pid), "A genuine lingering child remains alive without timeout killing")
	var delayed_reaped: bool = await _wait_reaped(delayed_pid)
	_check(delayed_reaped and _errors.is_empty() and OS.get_process_exit_code(delayed_pid) == 0, "Lingering child completes and is reaped")
	var main: MainScript = MainScript.new()
	main.startup_seed_text = "827364"
	main.startup_dimensions = Vector2i(1200, 800)
	main.startup_attempts = 96
	root.add_child(main)
	await process_frame
	OS.set_environment("BGR_HELP_OPENER_STATUS", "7")
	var before_main: int = _records().size()
	main._on_help_action(0)
	var deadline: int = Time.get_ticks_usec() + 2000000
	while _records().size() == before_main and Time.get_ticks_usec() < deadline:
		await process_frame
	var rows: Array = _records()
	var main_pid: int = int(rows[-1]["pid"]) if rows.size() == before_main + 1 else -1
	var weak_main: WeakRef = weakref(main)
	main.queue_free()
	await process_frame
	var main_reaped: bool = await _wait_reaped(main_pid)
	_check(main_pid > 0 and main_reaped and weak_main.get_ref() == null and OS.get_process_exit_code(main_pid) == 7, "SceneTree-owned reap survives actual Main destruction and invalid callback")
	for key: String in saved:
		if saved[key]["present"]:
			OS.set_environment(key, str(saved[key]["value"]))
		else:
			OS.unset_environment(key)
	print("HELP_OPENER_BOUNDARY_RESULT " + JSON.stringify({"status": "PASS" if _failures.is_empty() else "FAIL", "checks": _checks, "failures": _failures, "actual_opener_records": _records().size(), "real_browsers": 0, "Source_payload_changes": 0}))
	quit(0 if _failures.is_empty() else 1)


func _pid_present(pid: int) -> bool:
	return pid > 0 and FileAccess.file_exists("/proc/" + str(pid) + "/stat")


func _wait_reaped(pid: int) -> bool:
	if pid <= 0:
		return false
	var deadline: int = Time.get_ticks_usec() + 5000000
	while _pid_present(pid) and Time.get_ticks_usec() < deadline:
		await create_timer(0.05, true, false, true).timeout
	return not _pid_present(pid)


func _records() -> Array:
	var rows: Array = []
	if FileAccess.file_exists(_log):
		for line: String in FileAccess.get_file_as_string(_log).split("\n", false):
			rows.append(JSON.parse_string(line))
	return rows


func _write_fixture(path: String, text: String, permissions: int) -> bool:
	var file: FileAccess = FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		return false
	file.store_string(text)
	file.close()
	return FileAccess.set_unix_permissions(path, permissions) == OK


func _capture_error(message: String) -> void:
	_errors.append(message)


func _check(value: bool, description: String) -> void:
	_checks += 1
	if not value:
		_failures.append(description)
