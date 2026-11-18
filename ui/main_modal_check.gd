extends "res://ui/source_shortcuts_boundary_check.gd"

var _expected: Dictionary
var _output: String
var _close_events: int = 0
var _cancel_events: int = 0


func _initialize() -> void:
	await process_frame
	var expected_path: String = OS.get_environment("BGR_MAIN_MODAL_EXPECTED")
	_output = OS.get_environment("BGR_MAIN_MODAL_OUTPUT")
	if not _check(expected_path.is_absolute_path() and FileAccess.file_exists(expected_path), "Require an external independent modal catalog"):
		return
	if not _check(_output.is_absolute_path() and DirAccess.dir_exists_absolute(_output), "Require an existing private fixture directory"):
		return
	var owned_profile: String = OS.get_environment("BGR_TEST_ROOT")
	if not _check(owned_profile.is_absolute_path() and OS.get_user_data_dir().begins_with(owned_profile + "/"), "Require a private user-data profile"):
		return
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(expected_path))
	if not _check(parsed is Dictionary and parsed.get("locales") is Dictionary and parsed["locales"].size() == LANGUAGES.size(), "Require six independent locale catalogs"):
		return
	_expected = parsed["locales"]
	_simulation = BgrSimulation.new()
	if not _check(bool(_simulation.preferences_initialize()["ok"]), "Initialize the owned preference profile"):
		return
	_application = Main.new()
	_application.set("startup_seed_text", "827364")
	_application.set("startup_dimensions", Vector2i(96, 96))
	_application.set("startup_attempts", 0)
	if not _check(bool(_application.call("initialize_session", _simulation)), "Initialize the actual Main with the actual simulation"):
		return
	root.add_child(_application)
	_application.set_process(false)
	await process_frame
	var initial: Dictionary = _simulation.snapshot()
	if not _check(bool(initial["paused"]) and int(initial["tick"]) == 0 and int(initial["population"]) == 0, "Start with a paused empty world"):
		return
	for index: int in range(LANGUAGES.size()):
		var language: String = LANGUAGES[index]
		var model_before: Dictionary = _simulation.snapshot()
		if not _check(_expected.get(language) is Dictionary, "Missing independent locale: " + language):
			return
		if not _locale(index) or not _decision_dialogs(language, index == 0) or not _overwrite_dialog(language) or not _automatic_status(language):
			return
		if index == 0 and not _unavailable_save_fallback(language):
			return
		var selected_path: String = _output.path_join("modal-" + language + "-100%.bgw")
		if not _check(_simulation.snapshot() == model_before and str(_application.get("_world_path")) == selected_path, "Locale and presentation refresh must retain the model and selected file owner"):
			return
	if not (await _preference_failure_lifecycle()):
		return
	_finish(true)


func _decision_dialogs(language: String, exercise_quit_save: bool) -> bool:
	var row: Dictionary = _expected[language]
	var before: Dictionary = _capture()
	for operation: String in ["quit", "create", "reset"]:
		if not bool(_application.call("_request_world_decision", operation, PackedInt64Array([827364, 96, 96, 0]))):
			return _check(false, "Open the actual " + operation + " decision")
		var dialog: ConfirmationDialog = _application.get("_decision_dialog") as ConfirmationDialog
		var expected: Dictionary = row["decision"][operation]
		if not _check(dialog.visible and dialog.title == str(expected["title"]) and dialog.dialog_text == str(expected["body"]), "Localize decision copy: " + language + "/" + operation):
			return false
		if not _check(dialog.get_ok_button().text == str(expected["confirm"]) and dialog.get_cancel_button().text == str(expected["cancel"]) and (_application.get("_save_world_button") as Button).text == str(expected["save"]), "Localize decision controls: " + language + "/" + operation):
			return false
		dialog.get_cancel_button().pressed.emit()
		if not _check(_capture() == before and not dialog.visible, "Decision cancellation retains model and settings: " + operation):
			return false
		if operation == "quit" and exercise_quit_save:
			if not bool(_application.call("_request_world_decision", operation)):
				return _check(false, "Reopen the Quit decision for its Save path")
			_application.get("_save_world_button").pressed.emit()
			if not _check(str(_application.get("_file_operation")) == "world_save" and (_application.get("_file_dialog") as FileDialog).visible, "Quit Save must open the actual file owner"):
				return false
			_application.call("_finish_file_operation")
			if not _check(_capture() == before and not bool(_application.get("_shutting_down")), "Cancelling Quit Save retains the live world"):
				return false
		elif operation != "quit":
			if not bool(_application.call("_request_world_decision", operation, PackedInt64Array([827364, 96, 96, 0]))):
				return _check(false, "Reopen " + operation + " for its Save path")
			_application.get("_save_world_button").pressed.emit()
			var file_dialog: FileDialog = _application.get("_file_dialog") as FileDialog
			if file_dialog.visible:
				if str(_application.get("_file_operation")) != "world_save" or str(_application.get("_decision_operation")) != operation:
					return _check(false, "Save must retain the pending decision while choosing a file")
				var save_path: String = _output.path_join("decision-" + operation + ".bgw")
				_application.call("_on_genotype_file_selected", save_path)
			var saved: String = str(_application.get("_world_path"))
			if not _check(str(_application.get("_decision_operation")).is_empty() and FileAccess.file_exists(saved) and _simulation.snapshot() == before["world"], "Save before " + operation + " retains the current world"):
				return false
			before = _capture()
	return true


func _overwrite_dialog(language: String) -> bool:
	var row: Dictionary = _expected[language]["overwrite"]
	var filename: String = "modal-" + language + "-100%.bgw"
	var fixture: String = _output.path_join(filename)
	if not _check(not FileAccess.file_exists(fixture) and not DirAccess.dir_exists_absolute(fixture), "Overwrite fixture starts absent"):
		return false
	var created: Dictionary = _simulation.save_world_file(fixture)
	if not _check(bool(created["ok"]), "Create the overwrite fixture with the actual writer"):
		return false
	var prior: PackedByteArray = FileAccess.get_file_as_bytes(fixture)
	var prior_model: Dictionary = _simulation.snapshot()
	var old_path: String = str(_application.get("_world_path"))
	_application.call("_on_world_action", 13)
	_application.call("_on_genotype_file_selected", fixture)
	var dialog: ConfirmationDialog = _application.get("_overwrite_dialog") as ConfirmationDialog
	if not _check(dialog.visible and dialog.title == str(row["title"]) and dialog.dialog_text == str(row["body"]) + "\n" + filename, "Localize the actual overwrite confirmation and retain its literal filename: " + language):
		return false
	if not _check(dialog.get_ok_button().text == str(row["confirm"]) and dialog.get_cancel_button().text == str(row["cancel"]), "Localize actual overwrite buttons: " + language):
		return false
	dialog.get_cancel_button().pressed.emit()
	if not _check(FileAccess.get_file_as_bytes(fixture) == prior and _simulation.snapshot() == prior_model and str(_application.get("_world_path")) == old_path, "Overwrite cancellation retains bytes, model and filename"):
		return false
	_application.call("_on_world_action", 13)
	_application.call("_on_genotype_file_selected", fixture)
	dialog.get_ok_button().pressed.emit()
	return _check(FileAccess.file_exists(fixture) and _simulation.snapshot() == prior_model and str(_application.get("_world_path")) == fixture, "Confirmed Save As replaces only the selected file and adopts its filename")


func _automatic_status(language: String) -> bool:
	var cases: Dictionary = _expected[language]["automatic"]
	var prior_world_path: String = str(_application.get("_world_path"))
	var backup_path: String = _output.path_join("opened 100%@12345.bgw")
	var common: Dictionary = {"mode": "managed", "enabled": true, "last": {}, "source_save_pending": false, "error": "", "replacement_committed": false, "recovery_warning": "", "directory": _output}
	var variants: Array[Dictionary] = [
		{"name": "managed", "status": common.duplicate(true), "world_path": ""},
		{"name": "adjacent", "status": _status_variant(common, {"mode": "adjacent"}), "world_path": ""},
		{"name": "disabled", "status": _status_variant(common, {"enabled": false}), "world_path": ""},
		{"name": "last_save", "status": _status_variant(common, {"last": {"time": 9223372036854775806}}), "world_path": ""},
		{"name": "source_pending", "status": _status_variant(common, {"source_save_pending": true}), "world_path": ""},
		{"name": "raw_error", "status": _status_variant(common, {"error": "raw diagnostic: errno 17; literal %s, 100%"}), "world_path": ""},
		{"name": "disabled_error", "status": _status_variant(common, {"enabled": false, "error": "raw diagnostic: errno 17; literal %s, 100%"}), "world_path": ""},
		{"name": "committed", "status": _status_variant(common, {"replacement_committed": true}), "world_path": ""},
		{"name": "recovery_warning", "status": _status_variant(common, {"recovery_warning": "raw recovery detail: " + _output.path_join("recover 100%.bgw")}), "world_path": ""},
		{"name": "timestamp_warning", "status": _status_variant(common, {"mode": "adjacent"}), "world_path": backup_path},
		{"name": "combined", "status": _status_variant(common, {"mode": "adjacent", "last": {"time": 9223372036854775806}, "source_save_pending": true, "error": "raw diagnostic: errno 17; literal %s, 100%", "replacement_committed": true, "recovery_warning": "raw recovery detail: " + _output.path_join("recover 100%.bgw")}), "world_path": backup_path}
	]
	for item: Dictionary in variants:
		var name: String = str(item["name"])
		var expected: Dictionary = cases[name]
		_application.set("_world_path", str(item["world_path"]))
		_application.call("_update_automatic_status", item["status"])
		var label: Label = _application.get("_automatic_label") as Label
		var expected_text: String = str(expected["text"]).replace("{recovery_directory}", _output)
		var expected_tooltip: String = str(expected["tooltip"]).replace("{recovery_directory}", _output)
		if not _check(label.text == expected_text and label.tooltip_text == expected_tooltip and label.visible == bool(expected["visible"]), "Automatic status localization preserves raw data: " + language + "/" + name):
			return false
		if not _check((_application.get("_automatic_status") as Dictionary) == item["status"], "Store the original status dictionary without mutation: " + name):
			return false
	_application.set("_world_path", prior_world_path)
	return true


func _unavailable_save_fallback(language: String) -> bool:
	var before: Dictionary = _capture()
	var connections: Array = _application.get_signal_connection_list(&"world_save_requested")
	var owned: Callable
	for connection: Dictionary in connections:
		var candidate: Callable = connection["callable"]
		if candidate.get_object() == _application and candidate.get_method() == "_save_world_for_decision":
			owned = candidate
			break
	if not _check(owned.is_valid() and connections.size() == 1, "Capture Main's exact save callback before the missing-callback case"):
		return false
	_application.disconnect(&"world_save_requested", owned)
	_application.call("_request_world_decision", "create", PackedInt64Array([827364, 96, 96, 0]))
	_application.call("_world_decision_action", &"save")
	var error_text: String = str(_expected[language]["preference_failure"]["error"]) % str(_expected[language]["world_save_unavailable"])
	if not _check(str(_application.get("_decision_operation")).is_empty() and (_application.get("_error_label") as Label).text == error_text and _simulation.snapshot() == before["world"] and _simulation.settings_snapshot() == before["settings"], "Localize the unavailable-save fallback without changing the active world or settings"):
		_application.connect(&"world_save_requested", owned)
		return false
	_application.connect(&"world_save_requested", owned)
	var restored: Array = _application.get_signal_connection_list(&"world_save_requested")
	if not _check(restored.size() == connections.size() and restored[0]["callable"] == owned, "Restore Main's original save callback"):
		return false
	if not _locale(1):
		return false
	var changed_language_error: String = str(_expected["es"]["preference_failure"]["error"]) % str(_expected["es"]["world_save_unavailable"])
	return _check((_application.get("_error_label") as Label).text == changed_language_error, "Locale refresh must re-render the cached fallback key")


func _status_variant(source: Dictionary, changes: Dictionary) -> Dictionary:
	var result: Dictionary = source.duplicate(true)
	for key: String in changes:
		result[key] = changes[key]
	return result


func _preference_failure_lifecycle() -> bool:
	var preferences_path: String = OS.get_user_data_dir().path_join("biogenesis-r.preferences.xml")
	if not _check(not FileAccess.file_exists(preferences_path) and not DirAccess.dir_exists_absolute(preferences_path), "Use a fresh private preference profile"):
		return false
	if not _check(DirAccess.make_dir_recursive_absolute(preferences_path) == OK, "Create an owned preference destination failure"):
		return false
	_application.connect(&"simulation_closed", _record_close)
	_application.connect(&"close_cancelled", _record_cancel)
	_application.call("request_stop")
	var dialog: ConfirmationDialog = _application.get("_preference_failure") as ConfirmationDialog
	if not _check(dialog.visible and str(_application.get("_preference_close_kind")) == "stop", "Show recovery choices after preference save failure"):
		return false
	var raw_detail: String = str(_application.get("_error_message"))
	if not _check(not raw_detail.is_empty() and raw_detail == str(_application.get("_preference_failure_detail")), "Cache the raw preference write diagnostic"):
		return false
	for language: String in LANGUAGES:
		var row: Dictionary = _expected[language]["preference_failure"]
		var before_settings: Dictionary = _simulation.settings_snapshot()
		_application.call("_settings_locale_changed", {"language": language})
		var expected_body: String = str(row["error"]) % raw_detail + "\n" + str(row["help"])
		var discard: Button = dialog.find_child("DiscardPreferenceSave", true, false) as Button
		if not _check(dialog.title == str(row["title"]) and dialog.dialog_text == expected_body and str(_application.get("_error_message")) == raw_detail and dialog.get_ok_button().text == str(row["retry"]) and dialog.get_cancel_button().text == str(row["cancel"]) and is_instance_valid(discard) and discard.text == str(row["continue"]), "Refresh preference recovery copy while retaining the raw diagnostic and settings owner: " + language):
			return false
		if not _check(_simulation.settings_snapshot()["values"] == before_settings["values"], "Locale refresh retains accepted preference values"):
			return false
		dialog.get_ok_button().pressed.emit()
		if not _check(dialog.visible and str(_application.get("_preference_close_kind")) == "stop" and _close_events == 0, "Retry retains the live session when the owned destination still fails"):
			return false
	dialog.get_cancel_button().pressed.emit()
	if not _check(_cancel_events == 1 and _close_events == 0 and not _simulation.snapshot().is_empty(), "Cancel returns to the active simulation"):
		return false
	_application.call("request_stop")
	if not _check(dialog.visible and str(_application.get("_preference_close_kind")) == "stop", "Reopen recovery while the owned write failure persists"):
		return false
	var discard_button: Button = dialog.find_child("DiscardPreferenceSave", true, false) as Button
	if not _check(is_instance_valid(discard_button), "Locate the actual Continue button"):
		return false
	discard_button.pressed.emit()
	var close_deadline: int = Time.get_ticks_msec() + 3500
	while (_close_events == 0 or _application.get("_simulation") != null) and Time.get_ticks_msec() < close_deadline:
		await process_frame
	if not _check(_close_events == 1 and not dialog.visible and bool(_application.get("_shutting_down")) and _application.get("_simulation") == null and (_application.get("_state") as Dictionary).is_empty(), "Continue without saving closes once after terminal world release"):
		return false
	return _check(DirAccess.remove_absolute(preferences_path) == OK, "Remove the owned failure fixture")


func _record_close() -> void:
	_close_events += 1


func _record_cancel() -> void:
	_cancel_events += 1


func _capture() -> Dictionary:
	return {"world": _simulation.snapshot(), "settings": _simulation.settings_snapshot(), "path": str(_application.get("_world_path")), "status": (_application.get("_automatic_status") as Dictionary).duplicate(true)}


func _finish(passed: bool, message: String = "") -> void:
	if _finished:
		return
	_finished = true
	if is_instance_valid(_application):
		_application.call("shutdown")
		_application.queue_free()
	var failures: Array[String] = []
	if not passed:
		failures.append(message)
	print("BGR_MAIN_MODAL_BOUNDARY " + JSON.stringify({"status": "PASS" if passed else "FAIL", "failures": failures, "checks": _checks, "locales": LANGUAGES.size()}))
	quit(0 if passed else 1)
