extends SceneTree

const VersionDialog = preload("res://ui/version_dialog.gd")
const VersionFixture = preload("res://ui/version_check_fixture.gd")
const MainFixture = preload("res://ui/version_check_main_fixture.gd")
const HelpAboutScript = preload("res://ui/help_about.gd")
const LOCALES: Array[String] = ["en", "es", "ca", "de", "pt", "ru"]

var _checks: int = 0
var _failures: Array[String] = []
var _expected: Dictionary = {}
var _run: String
var _language: String = "es"
var _transport: Array[Dictionary] = []
var _closed: int = 0


func _initialize() -> void:
	_run_checks.call_deferred()


func _run_checks() -> void:
	_run = OS.get_environment("BGR_VERSION_TEST_RUN")
	var ready: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(_run.path_join("server-ready.json")))
	var base: String = OS.get_environment("BGR_VERSION_TEST_BASE_URL")
	if not _run.is_absolute_path() or not DirAccess.dir_exists_absolute(_run) or str(ready.get("host", "")) != "127.0.0.1" or base != "http://127.0.0.1:" + str(int(ready.get("port", 0))) or OS.get_environment("BGR_VERSION_TEST_UNAVAILABLE_PORT") != str(int(ready.get("unavailable_port", 0))):
		push_error("Require the owned loopback fixture and exact supervisor bindings")
		quit(2)
		return
	_expected = JSON.parse_string(FileAccess.get_file_as_string("res://ui/version_check_expected.json"))
	for example: Array in _expected["format"]:
		_check(VersionDialog.translate_version(str(example[0])) == str(example[1]), "Independent formatter example: " + JSON.stringify(example[0]))
	for locale: String in LOCALES:
		var messages: Dictionary = _expected["locales"][locale]
		for key: String in ["T_CHECK_LAST_VERSION_MNEMONIC", "T_CHECK_LAST_VERSION", "T_YOUR_VERSION", "T_LAST_VERSION", "T_CANT_CHECK_CURRENT_VERSION", "T_VERSION_TITLE"]:
			_check(HelpAboutScript.message(key, locale) == str(messages[key]), locale + ": exact Source " + key)
	await _main_integration()
	var success: Array[Array] = [["en", "/line-crlf", "1.0.1"], ["es", "/line-lf", "1.0"], ["ca", "/line-cr", "0.9"], ["de", "/empty-line", "0.0"], ["pt", "/bom-multiline", String.chr(0xfeff) + "900"], ["ru", "/hold-locale", "1.0"]]
	for item: Array in success:
		await _response(str(item[0]), str(item[1]), str(item[2]), -1)
	var errors: Array[Array] = [["en", "/eof", HTTPRequest.RESULT_SUCCESS], ["es", "/http-error", HTTPRequest.RESULT_SUCCESS], ["ca", "/oversize", HTTPRequest.RESULT_BODY_SIZE_LIMIT_EXCEEDED], ["de", "/redirect", HTTPRequest.RESULT_REDIRECT_LIMIT_REACHED], ["pt", "/unavailable", -2], ["ru", "/timeout", HTTPRequest.RESULT_TIMEOUT], ["en", "/invalid-utf8", HTTPRequest.RESULT_SUCCESS], ["es", "/nul", HTTPRequest.RESULT_SUCCESS]]
	for item: Array in errors:
		await _response(str(item[0]), str(item[1]), "", int(item[2]))
	await _cancel_in_flight()
	await _backup_deferral()
	_check(_requests("/redirect") > 1 and _requests("/redirect") <= 4, "Redirect loop stayed owned and within three redirects")
	print("VERSION_CHECK_BOUNDARY_RESULT " + JSON.stringify({"status": "PASS" if _failures.is_empty() else "FAIL", "checks": _checks, "formatter_cases": 32, "source_strings": 36, "successes": 6, "errors": 8, "cancellations": 1, "main_integrations": 1, "backup_deferrals": 2, "about_deferrals": 1, "transports": _transport, "failures": _failures, "production_requests": 0, "native_input_proven": false}))
	quit(0 if _failures.is_empty() else 1)


func _message(locale: String, key: String) -> String:
	return str(_expected["locales"][locale][key])


func _current_language() -> String:
	return _language


func _capture(target: Array[Dictionary], result: int, code: int, _headers: PackedStringArray, body: PackedByteArray) -> void:
	target.append({"result": result, "http": code, "body_bytes": body.size()})


func _count_closed() -> void:
	_closed += 1


func _response(locale: String, path: String, translated: String, expected_error: int) -> void:
	var dialog: VersionFixture = VersionFixture.new()
	dialog.case_path = path
	dialog.language = locale
	if path == "/hold-locale":
		_language = "es"
		dialog.language_provider = _current_language
	root.add_child(dialog)
	var events: Array[Dictionary] = []
	var request: HTTPRequest = dialog.find_child("VersionHTTPRequest", true, false) as HTTPRequest
	request.request_completed.connect(func(result: int, code: int, headers: PackedStringArray, body: PackedByteArray) -> void: _capture(events, result, code, headers, body))
	var pending_locale: String = "es" if path == "/hold-locale" else locale
	_check(dialog.title == _message(pending_locale, "T_VERSION_TITLE") and dialog.get_ok_button().text == _message(pending_locale, "T_CANCEL"), path + ": localized pending title and Cancel")
	_check(request.timeout == 10.0 and request.body_size_limit == 4096 and request.max_redirects == 3 and not request.accept_gzip, path + ": real request limits")
	if path == "/hold-locale":
		await _wait_for(func() -> bool: return _requests(path) == 1, 3.0, path + ": request reached owned server")
		_language = locale
		_write_release("release-hold-locale")
	await _wait_for(func() -> bool: return not events.is_empty(), 14.0, path + ": real request_completed")
	if events.is_empty():
		dialog.close_check()
		await process_frame
		return
	var event: Dictionary = events[0].duplicate()
	event["path"] = path
	_transport.append(event)
	_check(events.size() == 1, path + ": one completion")
	var text: Label = dialog.find_child("VersionCheckText", true, false) as Label
	if expected_error == -1:
		_check(int(event["result"]) == HTTPRequest.RESULT_SUCCESS and int(event["http"]) == 200, path + ": successful actual transport")
		_check(text.text == _message(locale, "T_YOUR_VERSION") + "0.9\n" + _message(locale, "T_LAST_VERSION") + translated, path + ": exact Source first line and local version")
	else:
		_check(int(event["result"]) != HTTPRequest.RESULT_SUCCESS if expected_error == -2 else int(event["result"]) == expected_error, path + ": actual error category")
		_check(text.text.begins_with(_message(locale, "T_CANT_CHECK_CURRENT_VERSION")) and text.text.length() > _message(locale, "T_CANT_CHECK_CURRENT_VERSION").length(), path + ": visible localized error with detail")
	_check(dialog.title == _message(locale, "T_VERSION_TITLE") and dialog.get_ok_button().text == _message(locale, "T_OK"), path + ": completed locale and OK")
	var closed_before: int = _closed
	dialog.closed.connect(_count_closed)
	dialog.confirmed.emit()
	await process_frame
	_check(_closed == closed_before + 1 and not is_instance_valid(dialog), path + ": confirmation closed once")


func _main_integration() -> void:
	var simulation: BgrSimulation = BgrSimulation.new()
	_check(bool(simulation.preferences_initialize()["ok"]), "Owned XDG initializes before Main")
	var main: MainFixture = MainFixture.new()
	main.startup_dimensions = Vector2i(800, 700)
	main.startup_attempts = 0
	_check(main.initialize_session(simulation), "Main accepts initialized owned session")
	root.add_child(main)
	await process_frame
	await process_frame
	_check(_all_requests().is_empty(), "No Main startup HTTP traffic before explicit action")
	var help: MenuButton = main.find_child("HelpMenu", true, false) as MenuButton
	var popup: PopupMenu = help.get_popup()
	_check(popup.item_count == 3 and popup.get_item_id(0) == 0 and popup.get_item_id(1) == 2 and popup.get_item_id(2) == 1, "Source Help order preserves action IDs")
	var pause: Button = main.find_child("PauseButton", true, false) as Button
	if bool(simulation.snapshot()["paused"]):
		pause.pressed.emit()
	var before: Dictionary = simulation.snapshot()
	var settings: Dictionary = simulation.settings_snapshot()
	var preferences: Dictionary = simulation.preferences_snapshot()
	var configuration: PackedInt64Array = main.applied_configuration()
	popup.id_pressed.emit(2)
	var dialog: Window = main.find_child("HelpVersionCheckDialog", true, false) as Window
	var identity: int = dialog.get_instance_id()
	popup.id_pressed.emit(2)
	_check(main.find_child("HelpVersionCheckDialog", true, false).get_instance_id() == identity, "Repeated real Help action keeps one dialog")
	await _wait_for(func() -> bool: return _requests("/duplicate") == 1, 3.0, "One owned request from duplicate actions")
	_check(not bool(simulation.snapshot()["paused"]) and not paused, "Pending dialog leaves running simulation and tree active")
	_check(simulation.settings_snapshot() == settings and simulation.preferences_snapshot() == preferences and main.applied_configuration() == configuration, "Version action preserves settings, preferences and world configuration")
	var after: Dictionary = simulation.snapshot()
	_check(after["width"] == before["width"] and after["height"] == before["height"] and after["seed"] == before["seed"] and int(after["tick"]) >= int(before["tick"]), "Running owner identity remains intact while autonomous time advances")
	_write_release("release-duplicate")
	await _wait_for(func() -> bool: return (dialog.find_child("VersionCheckClose", true, false) as Button).text == _message("en", "T_OK"), 3.0, "Main owned response completes")
	(dialog as AcceptDialog).confirmed.emit()
	await process_frame
	_check(main.find_child("HelpVersionCheckDialog", true, false) == null, "Main clears completed dialog ownership")
	main.shutdown()
	main.queue_free()
	await process_frame


func _cancel_in_flight() -> void:
	var dialog: VersionFixture = VersionFixture.new()
	dialog.case_path = "/cancel"
	root.add_child(dialog)
	var events: Array[Dictionary] = []
	var request: HTTPRequest = dialog.find_child("VersionHTTPRequest", true, false) as HTTPRequest
	request.request_completed.connect(func(result: int, code: int, headers: PackedStringArray, body: PackedByteArray) -> void: _capture(events, result, code, headers, body))
	await _wait_for(func() -> bool: return _requests("/cancel") == 1, 3.0, "Cancellation occurs after actual request")
	var before: int = _closed
	dialog.closed.connect(_count_closed)
	dialog.close_check()
	dialog.close_check()
	await process_frame
	_check(_closed == before + 1 and not is_instance_valid(dialog), "In-flight close is idempotent and frees dialog")
	_write_release("release-cancel")
	await _wait_for(func() -> bool: return _server_finished("/cancel"), 3.0, "Owned delayed response finishes after cancellation")
	await process_frame
	await process_frame
	_check(events.is_empty() and root.find_child("VersionCheckText", true, false) == null, "Cancelled request has no late completion or replacement dialog")


func _backup_deferral() -> void:
	var simulation: BgrSimulation = BgrSimulation.new()
	_check(bool(simulation.preferences_initialize()["ok"]), "Backup fixture initializes owned preferences")
	var main: MainFixture = MainFixture.new()
	main.startup_dimensions = Vector2i(800, 700)
	main.startup_attempts = 0
	main.qa_case_path = "/backup"
	_check(main.initialize_session(simulation), "Backup fixture initializes the actual Main owner")
	root.add_child(main)
	main.set_process(false)
	main.call("_open_settings")
	var settings: Window = main.get("_settings_window") as Window
	(settings.get("_backup_mode") as OptionButton).select(0)
	(settings.get("_backup") as CheckBox).set_pressed_no_signal(true)
	var fields: Dictionary = settings.get("_data")
	var editors: Array[LineEdit] = settings.get("_editors")
	for field: Dictionary in fields["fields"]:
		if str(field["key"]) == "BACKUP_DELAY":
			editors[int(field["fixed_order"])].text = "2"
	settings.call("_submit")
	await process_frame
	_check(not bool(main.call("_settings_open")) and bool(simulation.settings_snapshot()["values"]["AUTO_BACKUP"]) and int(simulation.settings_snapshot()["automatic_backup_mode"]) == 0, "Real Settings commits the Source automatic backup policy")
	if bool(simulation.snapshot()["paused"]):
		main.call("_toggle_pause")
	main.call("_accept_result", simulation.advance(510))
	main.call("_toggle_pause")
	var help: MenuButton = main.find_child("HelpMenu", true, false) as MenuButton
	help.get_popup().id_pressed.emit(2)
	var dialog: AcceptDialog = main.find_child("HelpVersionCheckDialog", true, false) as AcceptDialog
	await _wait_for(func() -> bool: return _requests("/backup") == 1, 3.0, "Backup regression owns one actual pending version request")
	var result: Dictionary = simulation.apply_commands(PackedInt32Array([2, 2, 2, 2]))
	main.call("_accept_result", result)
	_check(bool(result["ok"]) and int(result["commands_completed"]) == 2 and int(result["tick"]) == 512 and bool(simulation.automatic_backup_status()["source_save_pending"]), "Public simulation commands reach the real unnamed Source save boundary")
	main.set_process(true)
	await process_frame
	await process_frame
	_check(str(main.get("_file_operation")).is_empty() and not (main.get("_file_dialog") as FileDialog).visible and bool(simulation.automatic_backup_status()["source_save_pending"]) and is_instance_valid(dialog), "Pending version defers Save As without losing the Source request")
	_write_release("release-backup")
	await _wait_for(func() -> bool: return dialog.get_ok_button().text == _message("en", "T_OK"), 3.0, "Backup regression completes owned transport")
	dialog.confirmed.emit()
	help.get_popup().id_pressed.emit(1)
	var about: Window = main.find_child("HelpAboutDialog", true, false) as Window
	await process_frame
	await process_frame
	_check(is_instance_valid(about) and about.visible and str(main.get("_file_operation")).is_empty() and not (main.get("_file_dialog") as FileDialog).visible and bool(simulation.automatic_backup_status()["source_save_pending"]), "About opened immediately after Version retains the pending Source save without a second modal")
	about.call("close_about")
	await _wait_for(func() -> bool: return str(main.get("_file_operation")) == "world_save_as", 3.0, "Deferred Source request enters actual Save As after About closes")
	var picker: FileDialog = main.get("_file_dialog") as FileDialog
	_check(picker != null and picker.visible, "Deferred Source Save As is visible")
	picker.canceled.emit()
	await process_frame
	_check(not bool(simulation.automatic_backup_status()["source_save_pending"]) and int(simulation.automatic_backup_status()["attempts"]) == 1 and str(main.get("_world_path")).is_empty(), "Real Save As cancellation latches its Source attempt")
	main.set_process(false)
	if bool(simulation.snapshot()["paused"]):
		main.call("_toggle_pause")
	main.call("_accept_result", simulation.advance(510))
	main.call("_toggle_pause")
	help.get_popup().id_pressed.emit(2)
	var second_version: AcceptDialog = main.find_child("HelpVersionCheckDialog", true, false) as AcceptDialog
	var second_result: Dictionary = simulation.apply_commands(PackedInt32Array([2, 2, 2, 2]))
	main.call("_accept_result", second_result)
	_check(is_instance_valid(second_version) and bool(second_result["ok"]) and int(second_result["commands_completed"]) == 2 and int(second_result["tick"]) == 1024 and bool(simulation.automatic_backup_status()["source_save_pending"]), "Second public Source boundary retains its pending request under Version alone")
	main.set_process(true)
	await process_frame
	second_version.call("close_check")
	await _wait_for(func() -> bool: return str(main.get("_file_operation")) == "world_save_as", 3.0, "Version-only close releases the next actual Source Save As")
	picker.canceled.emit()
	await process_frame
	_check(not bool(simulation.automatic_backup_status()["source_save_pending"]) and int(simulation.automatic_backup_status()["attempts"]) == 2 and str(main.get("_world_path")).is_empty(), "Second real Save As cancellation latches its distinct Source attempt")
	main.shutdown()
	main.queue_free()
	await process_frame


func _all_requests() -> Array[Dictionary]:
	var entries: Array[Dictionary] = []
	for line: String in FileAccess.get_file_as_string(_run.path_join("requests.jsonl")).split("\n", false):
		var value: Variant = JSON.parse_string(line)
		if value is Dictionary:
			entries.append(value)
	return entries


func _requests(path: String) -> int:
	var count: int = 0
	for event: Dictionary in _all_requests():
		if event.get("event") == "request" and event.get("path") == path:
			count += 1
	return count


func _server_finished(path: String) -> bool:
	for event: Dictionary in _all_requests():
		if event.get("path") == path and event.get("event") in ["response", "peer-closed"]:
			return true
	return false


func _write_release(name: String) -> void:
	var file: FileAccess = FileAccess.open(_run.path_join(name), FileAccess.WRITE)
	if file == null:
		_check(false, "Cannot release owned server: " + name)
		return
	file.store_string("release\n")
	file.close()


func _wait_for(predicate: Callable, seconds: float, description: String) -> void:
	var deadline: int = Time.get_ticks_msec() + int(seconds * 1000.0)
	while not bool(predicate.call()) and Time.get_ticks_msec() < deadline:
		await process_frame
	_check(bool(predicate.call()), description)


func _check(value: bool, description: String) -> void:
	_checks += 1
	if not value:
		_failures.append(description)
