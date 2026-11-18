extends SceneTree

const MainFixture = preload("res://ui/version_check_main_fixture.gd")
const PROFILE: String = '<map MAP_XML_VERSION="1.0"><entry key="VERSION" value="700"/><entry key="WORLD_WIDTH" value="800"/><entry key="WORLD_HEIGHT" value="700"/><entry key="INITIAL_ORGANISMS" value="0"/><entry key="LOCALE" value="en"/><entry key="AUTO_BACKUP" value="false"/></map>'

var _run: String
var _checks: int = 0
var _failures: Array[String] = []
var _closed: int = 0
var _completions: Array[Dictionary] = []


func _initialize() -> void:
	_run_check.call_deferred()


func _run_check() -> void:
	_run = OS.get_environment("BGR_VERSION_TEST_RUN")
	if not _run.is_absolute_path() or not DirAccess.dir_exists_absolute(_run):
		push_error("Require an existing owned server output")
		quit(2)
		return
	var ready: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(_run.path_join("server-ready.json")))
	var base: String = "http://127.0.0.1:" + str(int(ready.get("port", 0)))
	if str(ready.get("host", "")) != "127.0.0.1" or OS.get_environment("BGR_VERSION_TEST_BASE_URL") != base or OS.get_environment("BGR_VERSION_TEST_UNAVAILABLE_PORT") != str(int(ready.get("unavailable_port", 0))):
		push_error("Require exact supervisor-owned loopback ports")
		quit(2)
		return
	for name: String in ["XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_RUNTIME_DIR"]:
		var path: String = OS.get_environment(name).simplify_path()
		if not path.is_absolute_path() or not path.begins_with(_run.simplify_path() + "/"):
			push_error("Require an owned " + name)
			quit(2)
			return
	var profile: String = OS.get_user_data_dir().path_join("biogenesis-r.preferences.xml")
	if FileAccess.file_exists(profile) or DirAccess.dir_exists_absolute(profile) or not profile.simplify_path().begins_with(_run.simplify_path() + "/"):
		push_error("Preserve prior profiles and use only fresh owned preferences")
		quit(2)
		return
	if DirAccess.make_dir_recursive_absolute(profile.get_base_dir()) != OK:
		push_error("Cannot create owned preference directory")
		quit(2)
		return
	var file: FileAccess = FileAccess.open(profile, FileAccess.WRITE)
	if file == null:
		push_error("Cannot create owned preference fixture")
		quit(2)
		return
	file.store_string(PROFILE)
	file.close()
	var simulation: BgrSimulation = BgrSimulation.new()
	var initialized: Dictionary = simulation.preferences_initialize()
	var settings: Dictionary = simulation.settings_snapshot()
	_check(bool(initialized["ok"]) and str(settings["language"]) == "en" and settings["genesis"] == PackedInt64Array([800, 700, 0]) and not bool(settings["values"]["AUTO_BACKUP"]), "Owned Source-compatible preferences initialize the explicit small world")
	var main: MainFixture = MainFixture.new()
	main.startup_seed_text = "827364"
	main.startup_dimensions = Vector2i(800, 700)
	main.startup_attempts = 0
	main.qa_case_path = "/shutdown"
	_check(main.initialize_session(simulation), "Actual Main accepts the owned initialized session")
	root.add_child(main)
	_check(bool(simulation.snapshot()["world_ready"]) and main.applied_configuration() == PackedInt64Array([827364, 800, 700, 0]), "Actual Main starts only the explicit zero-organism world")
	var help: MenuButton = main.find_child("HelpMenu", true, false) as MenuButton
	help.get_popup().id_pressed.emit(2)
	var dialog: AcceptDialog = main.find_child("HelpVersionCheckDialog", true, false) as AcceptDialog
	var request: HTTPRequest = dialog.find_child("VersionHTTPRequest", true, false) as HTTPRequest
	var dialog_ref: WeakRef = weakref(dialog)
	var request_ref: WeakRef = weakref(request)
	dialog.closed.connect(_record_closed)
	request.request_completed.connect(_record_completed)
	if not (await _wait_for(func() -> bool: return _request_count() == 1 and _completions.is_empty() and dialog.visible and not bool(dialog.get("_finished")), "Actual held shutdown request is pending")):
		quit(1)
		return
	main.shutdown()
	await process_frame
	_check(dialog_ref.get_ref() == null and request_ref.get_ref() == null and _closed == 1, "Public shutdown cancels and frees the actual dialog and HTTPRequest once")
	_check(main.get("_simulation") == null and (main.get("_state") as Dictionary).is_empty() and not main.is_processing() and not bool(simulation.snapshot()["world_ready"]), "Public shutdown releases the Main world owner")
	var released: Dictionary = simulation.snapshot()
	main.shutdown()
	await process_frame
	_check(_closed == 1 and simulation.snapshot() == released and main.get("_simulation") == null, "A second public shutdown preserves released state")
	var flag: FileAccess = FileAccess.open(_run.path_join("release-shutdown"), FileAccess.WRITE)
	if flag == null:
		push_error("Cannot release owned server response")
		quit(2)
		return
	flag.store_string("release\n")
	flag.close()
	await _wait_for(_server_finished, "Owned response finishes after public shutdown")
	await process_frame
	await process_frame
	_check(_completions.is_empty() and root.find_child("HelpVersionCheckDialog", true, false) == null and not bool(simulation.snapshot()["world_ready"]), "No late completion, replacement dialog or world resurrection follows shutdown")
	main.queue_free()
	await process_frame
	_check(not is_instance_valid(main) and _request_count() == 1, "Main frees without another request")
	print("VERSION_SHUTDOWN_BOUNDARY_RESULT " + JSON.stringify({"status": "PASS" if _failures.is_empty() else "FAIL", "checks": _checks, "requests": _request_count(), "closed": _closed, "late_completions": _completions.size(), "public_Main_shutdown": true, "native_close_claim": false, "failures": _failures}))
	quit(0 if _failures.is_empty() else 1)


func _record_closed() -> void:
	_closed += 1


func _record_completed(result: int, code: int, _headers: PackedStringArray, body: PackedByteArray) -> void:
	_completions.append({"result": result, "http": code, "body_bytes": body.size()})


func _events() -> Array[Dictionary]:
	var events: Array[Dictionary] = []
	for line: String in FileAccess.get_file_as_string(_run.path_join("requests.jsonl")).split("\n", false):
		var event: Variant = JSON.parse_string(line)
		if event is Dictionary:
			events.append(event)
	return events


func _request_count() -> int:
	var count: int = 0
	for event: Dictionary in _events():
		if event.get("event") == "request" and event.get("path") == "/shutdown":
			count += 1
	return count


func _server_finished() -> bool:
	for event: Dictionary in _events():
		if event.get("path") == "/shutdown" and event.get("event") in ["response", "peer-closed"]:
			return true
	return false


func _wait_for(predicate: Callable, description: String) -> bool:
	var deadline: int = Time.get_ticks_msec() + 3500
	while not bool(predicate.call()) and Time.get_ticks_msec() < deadline:
		await process_frame
	return _check(bool(predicate.call()), description)


func _check(value: bool, description: String) -> bool:
	_checks += 1
	if not value:
		_failures.append(description)
	return value
