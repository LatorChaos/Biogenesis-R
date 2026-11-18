extends SceneTree

const LANGUAGES: Array[String] = ["en", "es", "ca", "de", "pt", "ru"]
const LauncherScene = preload("res://scenes/launcher.tscn")

var _checks: int = 0
var _finished: bool = false
var _failures: Array[String] = []
var _expected: Dictionary = {}
var _launcher: Control
var _main: Control
var _simulation: BgrSimulation


func _initialize() -> void:
	await process_frame
	var expected_path: String = OS.get_environment("BGR_LAUNCHER_SETTINGS_EXPECTED")
	var profile: String = OS.get_environment("BGR_TEST_ROOT")
	if not _check(expected_path.is_absolute_path() and FileAccess.file_exists(expected_path), "Require the independent launcher/settings catalog"):
		return
	if not _check(profile.is_absolute_path() and OS.get_user_data_dir().begins_with(profile + "/"), "Require an owned user-data profile before any fixture write"):
		return
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(expected_path))
	if not _check(parsed is Dictionary and parsed.get("locales") is Dictionary and parsed["locales"].size() == LANGUAGES.size(), "Require six independent locale catalogs"):
		return
	_expected = parsed["locales"]
	_launcher = LauncherScene.instantiate() as Control
	root.add_child(_launcher)
	await process_frame
	await process_frame
	_simulation = _launcher.get("_simulation") as BgrSimulation
	if not _check(is_instance_valid(_simulation), "Use the actual Launcher preferences owner"):
		return
	var initial_settings: Dictionary = _simulation.settings_snapshot()
	var language: String = str(initial_settings["language"])
	if not _check(language == "en" and initial_settings["genesis"] == PackedInt64Array([96, 96, 0]) and not bool(initial_settings["values"]["AUTO_BACKUP"]), "The isolated profile starts in English with a small empty world and automatic backups off"):
		return
	if not _check(_launcher_locale("en", false), "Localize the stopped Launcher"):
		return
	var start_button: Button = _launcher.get("_start_stop") as Button
	start_button.pressed.emit()
	await process_frame
	await process_frame
	_main = _launcher.get("_main") as Control
	if not _check(is_instance_valid(_main), "Start through the real Launcher button"):
		return
	var initial: Dictionary = _simulation.snapshot()
	if not _check(bool(initial["paused"]) and int(initial["population"]) == 0 and int(initial["tick"]) == 0 and initial.has("seed"), "Start a paused empty 96x96 world"):
		return
	for index: int in range(LANGUAGES.size()):
		var target: String = LANGUAGES[index]
		var current: String = str(_simulation.settings_snapshot()["language"])
		if current != target:
			var locale_applied: bool = await _apply_locale(index)
			if not locale_applied:
				return
		var settings_checked: bool = await _check_settings(target)
		if not settings_checked:
			return
		if not _launcher_locale(target, true):
			return
		if not _launcher_presentation(target):
			return
		var errors_checked: bool = await _main_errors(target)
		if not errors_checked:
			return
	var cancel_checked: bool = await _cancel_draft_retains_settings()
	if not cancel_checked:
		return
	start_button = _launcher.get("_start_stop") as Button
	start_button.pressed.emit()
	await process_frame
	await process_frame
	if not _check(not is_instance_valid(_launcher.get("_main")) and str((_launcher.get("_status") as Label).text) == str(_expected["ru"]["launcher"]["R_LAUNCHER_STOPPED"]), "Stop through the real Launcher button and retain the current locale"):
		return
	var imports_checked: bool = await _imports()
	if not imports_checked:
		return
	_finish(true)


func _apply_locale(index: int) -> bool:
	_main.call("_open_settings")
	await process_frame
	var window: Window = _main.get("_settings_window") as Window
	if not _check(is_instance_valid(window) and window.visible, "Open the actual Settings owner before changing locale"):
		return false
	var before: Dictionary = _simulation.snapshot()
	var settings: Dictionary = _simulation.settings_snapshot()
	var expected_values: Dictionary = (settings["values"] as Dictionary).duplicate(true)
	expected_values["LOCALE"] = LANGUAGES[index]
	expected_values["LOCALE_INDEX"] = index
	var locale: OptionButton = window.get("_locale") as OptionButton
	locale.select(index)
	(window.get("_ok") as Button).pressed.emit()
	await process_frame
	await process_frame
	var after_settings: Dictionary = _simulation.settings_snapshot()
	if not _check(str(after_settings["language"]) == LANGUAGES[index] and int(after_settings["locale"]) == index and int(after_settings["genesis"][0]) == 96 and int(after_settings["genesis"][1]) == 96 and int(after_settings["genesis"][2]) == 0, "Apply the real language choice without changing retained settings"):
		return false
	if not _check(_simulation.snapshot() == before and expected_values == after_settings["values"], "Locale change preserves all settings except the intended LOCALE and LOCALE_INDEX values"):
		return false
	return _check(str(_launcher.get("_language")) == LANGUAGES[index] and str(_main.get("_main_language")) == LANGUAGES[index], "The typed Main language signal refreshes Launcher")


func _launcher_locale(language: String, running: bool) -> bool:
	var row: Dictionary = _expected[language]
	var labels: Dictionary = row["launcher"]
	var button: Button = _launcher.get("_start_stop") as Button
	var import_button: Button = _launcher.get("_import_button") as Button
	var quit_button: Button = _launcher.get("_quit_button") as Button
	var status_key: String = "R_LAUNCHER_RUNNING" if running else "R_LAUNCHER_STOPPED"
	var expected_status: String = str(labels[status_key])
	var geometry_key: String = str(_launcher.get("_geometry_notice_key"))
	if running and not geometry_key.is_empty():
		expected_status += "\n" + str(labels[geometry_key])
	var ok: bool = _check(
		_launcher.get_window().title == str(labels["R_LAUNCHER_TITLE"])
		and button.text == str(labels["R_LAUNCHER_STOP"] if running else labels["R_LAUNCHER_START"])
		and button.tooltip_text == str(labels["R_LAUNCHER_ACTION_HELP"])
		and quit_button.text == str(labels["R_LAUNCHER_QUIT"])
		and import_button.text == str(labels["R_LAUNCHER_IMPORT"])
		and import_button.tooltip_text == str(labels["R_LAUNCHER_IMPORT_HELP"])
		and (_launcher.get("_status") as Label).text == expected_status,
		"Refresh Launcher title, actions, help, and status: " + language
	)
	if not ok:
		return false
	var dialog: FileDialog = _launcher.get("_import_dialog") as FileDialog
	var expected_filter: String = "*.xml ; " + str(labels["R_LAUNCHER_XML_FILTER"])
	var expected_all_files: String = "* ; " + str(row["all_files"])
	var filters: PackedStringArray = dialog.filters
	return _check(
		dialog.title == str(labels["R_LAUNCHER_IMPORT_TITLE"])
		and filters.size() == 2 and filters[0] == expected_filter and filters[1] == expected_all_files
		and str(labels["R_LAUNCHER_IMPORT_APPROVE"]) == dialog.ok_button_text
		and dialog.get_cancel_button().text == str(row["cancel"]),
		"Refresh Launcher import chooser labels: " + language
	)


func _check_settings(language: String) -> bool:
	_main.call("_open_settings")
	await process_frame
	var window: Window = _main.get("_settings_window") as Window
	if not _check(is_instance_valid(window) and window.visible, "Open the actual Settings window"):
		return false
	var row: Dictionary = _expected[language]
	var data: Dictionary = window.get("_data")
	var labels: Array = window.get("_labels")
	var editors: Array = window.get("_editors")
	var captions: Dictionary = row["captions"]
	for field: Dictionary in data["fields"]:
		var index: int = int(field["fixed_order"])
		var key: String = str(field["key"])
		var editor: LineEdit = editors[index] as LineEdit
		if not _check(str(captions[key]) == (labels[index] as Label).text and editor.accessibility_name == str(captions[key]) and editor.tooltip_text == key and editor.name == "Setting_" + key, "Localize Settings captions and accessibility while retaining raw key identifiers: " + language + "/" + key):
			return false
	var tab: TabContainer = window.get("_tabs") as TabContainer
	for index: int in range(7):
		var id: String = str(data["tabs"][index]["id"])
		if not _check(tab.get_tab_title(index) == str(row["settings_tabs"][id]), "Localize Settings tab: " + language + "/" + id):
			return false
	for key: String in row["settings_sections"]:
		var heading: Label = window.find_child(key, true, false) as Label
		if not _check(is_instance_valid(heading) and heading.text == str(row["settings_sections"][key]), "Localize Settings section: " + language + "/" + key):
			return false
	var source_labels: Dictionary = row["settings_source"]
	var extra: Dictionary = row["settings"]
	var backup: CheckBox = window.get("_backup") as CheckBox
	var mode: OptionButton = window.get("_backup_mode") as OptionButton
	var hardware: OptionButton = window.get("_hardware") as OptionButton
	var framebuffer: CheckBox = window.get("_fbo") as CheckBox
	var locale: OptionButton = window.get("_locale") as OptionButton
	var ok: bool = _check(
		window.title == str(source_labels["T_PARAMETERS_CONFIGURATION"])
		and (window.get("_language_label") as Label).text == str(source_labels["T_LANGUAGE"])
		and (window.get("_disclosure") as Label).text == str(extra["R_SETTINGS_DISCLOSURE"])
		and backup.text == str(source_labels["T_AUTOMATIC_BACKUPS"])
		and backup.tooltip_text == str(extra["R_SETTINGS_BACKUP_HELP"])
		and mode.get_item_text(0) == str(extra["R_SETTINGS_ADJACENT"])
		and mode.get_item_text(1) == str(extra["R_SETTINGS_MANAGED"])
		and mode.tooltip_text == str(extra["R_SETTINGS_BACKUP_MODE_HELP"])
		and (window.get("_graphics_label") as Label).text == str(source_labels["T_HARDWARE_ACCELERATION"])
		and (window.get("_graphics_note") as Label).text == str(extra["R_SETTINGS_GRAPHICS_NOTE"])
		and hardware.get_item_text(0) == str(source_labels["T_NONE"])
		and hardware.get_item_text(1) == str(source_labels["T_OPENGL"])
		and framebuffer.text == str(source_labels["T_DISABLE_FBOBJECT"])
		and (window.get("_ok") as Button).text == str(source_labels["T_OK"])
		and (window.get("_cancel") as Button).text == str(source_labels["T_CANCEL"])
		and (window.get("_defaults") as Button).text == str(source_labels["T_DEFAULT_VALUES"]),
		"Localize Settings controls and disclosures: " + language
	)
	if not ok:
		return false
	for index: int in range(6):
		if not _check(locale.get_item_id(index) == index and locale.get_item_text(index) == str(row["language_names"][LANGUAGES[index]]), "Retain Settings language choice ID and label: " + language):
			return false
	var settings_before_notice: Dictionary = _simulation.settings_snapshot()
	window.call("apply_locale", "pt_br")
	var notice: Label = window.get("_locale_notice") as Label
	var english_notice: String = str(_expected["en"]["settings"]["R_SETTINGS_UNSUPPORTED_LOCALE"]) % ["pt_br", int(settings_before_notice["locale"])]
	if not _check(notice.visible and notice.text == english_notice, "Retain unsupported stored-language warning and Source index: " + language):
		return false
	window.call("apply_locale", language)
	if not _check(_simulation.settings_snapshot() == settings_before_notice, "Refreshing unsupported-locale presentation does not mutate preferences"):
		return false
	(window.get("_cancel") as Button).pressed.emit()
	await process_frame
	return _check(not is_instance_valid(_main.get("_settings_window")), "Cancel closes the actual Settings owner")


func _cancel_draft_retains_settings() -> bool:
	_main.call("_open_settings")
	await process_frame
	var window: Window = _main.get("_settings_window") as Window
	var before: Dictionary = _simulation.settings_snapshot()
	var draft: BgrSettingsDraft = window.get("draft") as BgrSettingsDraft
	var snapshot: Dictionary = draft.snapshot()
	var width: int = (snapshot["keys"] as PackedStringArray).find("WORLD_WIDTH")
	var texts: PackedStringArray = snapshot["texts"]
	texts[width] = "uncommitted draft"
	var controls: Dictionary = {
		"texts": texts,
		"locale": snapshot["locale"],
		"automatic_backup": snapshot["automatic_backup"],
		"hardware": snapshot["hardware"],
		"disable_framebuffer": snapshot["disable_framebuffer"],
		"automatic_backup_mode": snapshot.get("automatic_backup_mode", 0),
	}
	if not _check(bool(draft.replace_controls(controls)["ok"]), "Change only the open Settings draft"):
		return false
	(window.get("_cancel") as Button).pressed.emit()
	await process_frame
	var after: Dictionary = _simulation.settings_snapshot()
	return _check(after == before and not is_instance_valid(_main.get("_settings_window")), "Cancel discards draft text without changing applied settings")


func _main_errors(language: String) -> bool:
	var row: Dictionary = _expected[language]
	var fields: Array[LineEdit] = [
		_main.get("_seed_input") as LineEdit,
		_main.get("_width_input") as LineEdit,
		_main.get("_height_input") as LineEdit,
		_main.get("_attempts_input") as LineEdit,
	]
	var keys: Array[String] = ["R_INTEGER_SEED", "R_INTEGER_WIDTH", "R_INTEGER_HEIGHT", "R_INTEGER_ATTEMPTS"]
	for index: int in range(fields.size()):
		var before_world: Dictionary = _simulation.snapshot()
		var before_settings: Dictionary = _simulation.settings_snapshot()
		var original: String = fields[index].text
		fields[index].text = "invalid value"
		_main.call("_request_create_world")
		await process_frame
		var expected: String = str(row["error_prefix"]) + str(row["main_errors"][keys[index]])
		if not _check(
			(_main.get("_error_label") as Label).visible
			and (_main.get("_error_label") as Label).text == expected
			and _main.get_viewport().gui_get_focus_owner() == fields[index]
			and fields[index].text == "invalid value"
			and _simulation.snapshot() == before_world
			and _simulation.settings_snapshot() == before_settings,
			"Reject and localize invalid setup value without changing its owner: " + language + "/" + keys[index]
		):
			return false
		fields[index].text = original
	var seed: LineEdit = fields[0]
	for value: String in ["9223372036854775808", "-9223372036854775809"]:
		var before_world: Dictionary = _simulation.snapshot()
		var before_settings: Dictionary = _simulation.settings_snapshot()
		var original: String = seed.text
		seed.text = value
		_main.call("_request_create_world")
		await process_frame
		var expected: String = str(row["error_prefix"]) + str(row["main_errors"]["R_INTEGER_RANGE"])
		if not _check(
			(_main.get("_error_label") as Label).text == expected
			and _main.get_viewport().gui_get_focus_owner() == seed
			and seed.text == value
			and _simulation.snapshot() == before_world
			and _simulation.settings_snapshot() == before_settings,
			"Reject signed 64-bit overflow without changing the world: " + language
		):
			return false
		seed.text = original
	return true


func _launcher_presentation(language: String) -> bool:
	var row: Dictionary = _expected[language]
	var labels: Dictionary = row["launcher"]
	var original_status: String = str(_launcher.get("_status_key"))
	var original_geometry: String = str(_launcher.get("_geometry_notice_key"))
	var original_result: Dictionary = (_launcher.get("_import_result_state") as Dictionary).duplicate(true)
	var world_before: Dictionary = _simulation.snapshot()
	var settings_before: Dictionary = _simulation.settings_snapshot()
	for committed: bool in [false, true]:
		var status: String = str(_launcher.call("_preference_error_status", {"replacement_committed": committed}))
		var key: String = "R_LAUNCHER_PREFS_DURABILITY" if committed else "R_LAUNCHER_PREFS_LOAD_FAILED"
		if not _check(status == str(labels[key]), "Localize cached preference error status: " + language + "/" + key):
			return false
		_launcher.call("_refresh_locale")
		if not _check(str((_launcher.get("_status") as Label).text) == str(labels[key]), "Refresh cached preference error status: " + language + "/" + key):
			return false
	for key: String in ["R_LAUNCHER_SCREEN_SMALL", "R_LAUNCHER_GEOMETRY_NORMAL"]:
		_launcher.set("_status_key", "R_LAUNCHER_RUNNING")
		_launcher.set("_geometry_notice_key", key)
		_launcher.call("_refresh_locale")
		if not _check(str((_launcher.get("_status") as Label).text) == str(labels["R_LAUNCHER_RUNNING"]) + "\n" + str(labels[key]), "Localize cached geometry status without formatter-derived expectations: " + language + "/" + key):
			return false
	var result_cases: Array[Dictionary] = [
		{"status": "R_LAUNCHER_IMPORTED", "title": "R_LAUNCHER_IMPORTED_TITLE", "dialog": "notice", "state": {"applied": true, "ok": true, "notice": "raw import notice"}},
		{"status": "R_LAUNCHER_IMPORT_ATTENTION", "title": "R_LAUNCHER_IMPORT_ATTENTION_TITLE", "dialog": "durability", "state": {"applied": true, "ok": false, "error": "raw durability diagnostic"}},
		{"status": "R_LAUNCHER_IMPORT_FAILED", "title": "R_LAUNCHER_IMPORT_FAILED_TITLE", "dialog": "retained", "state": {"applied": false, "ok": false, "error": "raw rejection diagnostic"}},
	]
	var result_dialog: AcceptDialog = _launcher.get("_import_result") as AcceptDialog
	for item: Dictionary in result_cases:
		var state: Dictionary = item["state"].duplicate(true)
		_launcher.set("_status_key", str(item["status"]))
		_launcher.set("_import_result_state", state)
		_launcher.call("_refresh_locale")
		var body: String = str(state.get("notice", ""))
		if str(item["dialog"]) == "durability":
			body = str(labels["R_LAUNCHER_IMPORT_DURABILITY"]) + "\n" + str(state["error"])
		elif str(item["dialog"]) == "retained":
			body = str(state["error"]) + "\n" + str(labels["R_LAUNCHER_IMPORT_RETAINED"])
		if not _check(
			str((_launcher.get("_status") as Label).text) == str(labels[str(item["status"])])
			and result_dialog.title == str(labels[str(item["title"])])
			and result_dialog.dialog_text == body
			and result_dialog.ok_button_text == str(labels["R_LAUNCHER_RESULT_OK"])
			and (_launcher.get("_import_result_state") as Dictionary) == state,
			"Refresh cached import result and retain raw diagnostics: " + language + "/" + str(item["status"])
		):
			return false
	_launcher.set("_status_key", original_status)
	_launcher.set("_geometry_notice_key", original_geometry)
	_launcher.set("_import_result_state", original_result)
	_launcher.call("_refresh_locale")
	return _check(_simulation.snapshot() == world_before and _simulation.settings_snapshot() == settings_before, "Presentation-only status fixtures do not mutate Core state")


func _imports() -> bool:
	var directory: String = OS.get_user_data_dir().path_join("launcher-settings-fixtures")
	var valid_path: String = directory.path_join("valid-import.xml")
	var invalid_path: String = directory.path_join("invalid-import.xml")
	if not _check(not DirAccess.dir_exists_absolute(directory), "Preserve pre-existing Launcher import fixtures"):
		return false
	if not _check(DirAccess.make_dir_recursive_absolute(directory) == OK, "Create only the owned import fixture directory"):
		return false
	var valid_text: String = '<map MAP_XML_VERSION="1.0"><entry key="VERSION" value="700"/><entry key="WORLD_WIDTH" value="96"/><entry key="WORLD_HEIGHT" value="96"/><entry key="INITIAL_ORGANISMS" value="0"/><entry key="LOCALE" value="DE"/></map>'
	var invalid_text: String = "preserved invalid import bytes %"
	for entry: Array in [[valid_path, valid_text], [invalid_path, invalid_text]]:
		var file: FileAccess = FileAccess.open(str(entry[0]), FileAccess.WRITE)
		if not _check(file != null, "Create an owned import fixture"):
			return false
		file.store_string(str(entry[1]))
		file.close()
	var import_button: Button = _launcher.get("_import_button") as Button
	var dialog: FileDialog = _launcher.get("_import_dialog") as FileDialog
	var result_dialog: AcceptDialog = _launcher.get("_import_result") as AcceptDialog
	var source_before: PackedByteArray = FileAccess.get_file_as_bytes(valid_path)
	var settings_before: Dictionary = _simulation.settings_snapshot()
	import_button.pressed.emit()
	if not _check(dialog.visible, "Open the actual legacy preferences chooser"):
		return false
	dialog.canceled.emit()
	await process_frame
	if not _check(FileAccess.get_file_as_bytes(valid_path) == source_before and _simulation.settings_snapshot() == settings_before, "Cancel import without reading or changing its source or settings"):
		return false
	import_button.pressed.emit()
	dialog.file_selected.emit(valid_path)
	await process_frame
	var success: Dictionary = (_launcher.get("_import_result_state") as Dictionary).duplicate(true)
	var success_row: Dictionary = _expected["de"]["launcher"]
	if not _check(
		bool(success.get("ok", false)) and bool(success.get("applied", false))
		and str(_launcher.get("_language")) == "de"
		and str((_launcher.get("_status") as Label).text) == str(success_row["R_LAUNCHER_IMPORTED"])
		and result_dialog.title == str(success_row["R_LAUNCHER_IMPORTED_TITLE"])
		and result_dialog.dialog_text == str(success["notice"])
		and FileAccess.get_file_as_bytes(valid_path) == source_before,
		"Apply a real legacy import and preserve its source bytes and notice"
	):
		return false
	result_dialog.confirmed.emit()
	await process_frame
	var native_before: PackedByteArray = FileAccess.get_file_as_bytes(OS.get_user_data_dir().path_join("biogenesis-r.preferences.xml"))
	var settings_before_reject: Dictionary = _simulation.settings_snapshot()
	var invalid_before: PackedByteArray = FileAccess.get_file_as_bytes(invalid_path)
	import_button.pressed.emit()
	dialog.file_selected.emit(invalid_path)
	await process_frame
	var rejected: Dictionary = (_launcher.get("_import_result_state") as Dictionary).duplicate(true)
	var reject_row: Dictionary = _expected["de"]["launcher"]
	if not _check(
		not bool(rejected.get("ok", true)) and not bool(rejected.get("applied", true))
		and result_dialog.dialog_text == str(rejected["error"]) + "\n" + str(reject_row["R_LAUNCHER_IMPORT_RETAINED"])
		and result_dialog.title == str(reject_row["R_LAUNCHER_IMPORT_FAILED_TITLE"])
		and FileAccess.get_file_as_bytes(invalid_path) == invalid_before
		and FileAccess.get_file_as_bytes(OS.get_user_data_dir().path_join("biogenesis-r.preferences.xml")) == native_before
		and _simulation.settings_snapshot() == settings_before_reject,
		"Reject a real malformed import while preserving raw error and both byte owners"
	):
		return false
	return true


func _check(condition: bool, message: String) -> bool:
	_checks += 1
	if not condition:
		_failures.append(message)
		printerr("BGR_LAUNCHER_SETTINGS_FAIL " + message)
		_finish(false)
	return condition


func _finish(passed: bool) -> void:
	if _finished:
		return
	_finished = true
	if is_instance_valid(_launcher):
		var active_main: Control = _launcher.get("_main") as Control
		if is_instance_valid(active_main):
			active_main.call("shutdown")
		_launcher.queue_free()
	print("BGR_LAUNCHER_SETTINGS_BOUNDARY " + JSON.stringify({
		"status": "PASS" if passed else "FAIL",
		"checks": _checks,
		"failures": _failures,
		"locales": LANGUAGES.size(),
		"settings_fields": 132,
	}))
	quit(0 if passed else 1)
