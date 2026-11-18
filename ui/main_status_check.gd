extends "res://ui/source_shortcuts_boundary_check.gd"

var _expected: Dictionary
const StatusLabels = preload("res://ui/main_labels.gd")


func _initialize() -> void:
	await process_frame
	var expected_path: String = OS.get_environment("BGR_MAIN_STATUS_EXPECTED")
	if expected_path.is_empty():
		expected_path = "res://ui/main_status_expected.json"
	var directory: String = OS.get_environment("BGR_MAIN_STATUS_OUTPUT")
	if not _check((expected_path.is_absolute_path() or expected_path.begins_with("res://")) and FileAccess.file_exists(expected_path) and directory.is_absolute_path() and DirAccess.dir_exists_absolute(directory), "Require independent labels and an existing private output directory"):
		return
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(expected_path))
	if not _check(parsed is Dictionary and parsed.size() == 6, "Independent expectations must cover six locales"):
		return
	_expected = parsed
	_simulation = BgrSimulation.new()
	_application = Main.new()
	_application.set("startup_seed_text", "827364")
	_application.set("startup_dimensions", Vector2i(96, 96))
	_application.set("startup_attempts", 0)
	if not _check(bool(_application.call("initialize_session", _simulation)), "Use the actual Main and Rust session"):
		return
	root.add_child(_application)
	_application.set_process(false)
	await process_frame
	var first: Dictionary = _simulation.snapshot()
	if not _check(bool(first["paused"]) and int(first["tick"]) == 0 and int(first["world_time"]) == 0 and int(first["population"]) == 0, "Initialize the owned paused empty world"):
		return
	var step: Button = _application.get("_step_button") as Button
	step.pressed.emit()
	var stepped: Dictionary = _simulation.snapshot()
	if not _check(int(stepped["tick"]) == 1 and int(stepped["world_time"]) == 0, "One actual step must distinguish Rust ticks from Source game time"):
		return
	var seed: LineEdit = _application.get("_seed_input") as LineEdit
	seed.text = "uncommitted seed"
	var canvas: Control = _application.get("_canvas") as Control
	canvas.call("zoom_at", 1.25 / float(_application.get("_view_zoom")), canvas.size * 0.5)
	var zoom: float = float(_application.get("_view_zoom"))
	# These cached presentation values exercise diagnostics without advancing the model
	_application.set("_stored_delay_ms", 0)
	_application.set("_last_step_ms", 12.5)
	_application.set("_pending_ticks", 17.0)
	_application.call("_update_labels")
	_application.call("_show_error", "owned diagnostic details")
	_application.call("_set_placement_mode", "random")
	_application.call("_open_settings")
	await process_frame
	var settings: Window = _application.get("_settings_window") as Window
	if not _check(is_instance_valid(settings) and settings.visible, "Keep the actual Settings owner open across locale changes"):
		return
	for index: int in range(LANGUAGES.size()):
		var language: String = LANGUAGES[index]
		if not _check(_expected.get(language) is Dictionary, "Missing independent locale: " + language):
			return
		var before: Dictionary = _simulation.snapshot()
		if not _number_cases(language) or not _locale(index) or not _presentation(language) or not _large_counters(language):
			return
		if not _check(_simulation.snapshot() == before and _application.get("_settings_window") == settings and seed.text == "uncommitted seed" and float(_application.get("_view_zoom")) == zoom, "Live presentation must retain the model, Settings owner, typed input and zoom"):
			return
		if not _check(int(_application.get("_stored_delay_ms")) == 0 and float(_application.get("_last_step_ms")) == 12.5, "Locale callback must retain the pre-commit timing presentation"):
			return
		_application.set("_runtime_failed", true)
		_application.call("_update_labels")
		if not _check((_application.get("_mode_label") as Label).text == str(_expected[language]["R_STOPPED_ERROR"]) and (_application.get("_pause_button") as Button).disabled and step.disabled, "Localized runtime errors retain control priority"):
			return
		_application.set("_runtime_failed", false)
		_application.call("_update_labels")
	settings.call("_cancel_draft")
	await process_frame
	_application.set("_stored_delay_ms", 2)
	_application.set("_last_step_ms", 0.0)
	_application.set("_pending_ticks", 0.0)
	_application.call("_set_placement_mode", "")
	_application.call("_update_labels")
	_finish(true)


func _presentation(language: String) -> bool:
	var labels: Dictionary = _expected[language]
	for pair: Array in [["SeedInputLabel", "R_SETUP_SEED"], ["WidthInputLabel", "R_SETUP_WIDTH"], ["HeightInputLabel", "R_SETUP_HEIGHT"], ["AttemptsInputLabel", "R_SETUP_ATTEMPTS"], ["ConversionDisclosure", "R_DISCLOSURE"]]:
		var label: Label = _application.find_child(str(pair[0]), true, false) as Label
		if not _check(is_instance_valid(label) and label.text == str(labels[str(pair[1])]), "Refresh the actual setup control: " + str(pair[0])):
			return false
	var seed: LineEdit = _application.get("_seed_input") as LineEdit
	if not _check(seed.accessibility_name == str(labels["R_SETUP_SEED"]) and (_application.find_child("SeedInputLabel", true, false) as Label).tooltip_text == str(labels["R_CREATE_SETUP_HINT"]), "Refresh setup accessibility and seed behavior disclosure"):
		return false
	if not _check((_application.get("_error_label") as Label).text == str(labels["R_ERROR"]) % "owned diagnostic details", "Translate feedback framing while retaining exact diagnostic details"):
		return false
	var state: Dictionary = _simulation.snapshot()
	var metrics: String = "   ".join(PackedStringArray([
		str(labels["T_TIME"]) + str(int(state["world_time"])),
		str(labels["R_TICK"]) + str(int(state["tick"])),
		str(labels["T_CURRENT_POPULATION"]) + str(int(state["population"])),
		str(labels["T_O2"]) + _number_text(float(state["oxygen"]), language, 2),
		str(labels["T_CO2"]) + _number_text(float(state["carbon_dioxide"]), language, 2),
		str(labels["T_CH4"]) + _number_text(float(state["methane"]), language, 2)
	]))
	if not _check((_application.get("_metrics_label") as Label).text == metrics, "Show Source game time and retain tick, population and gases"):
		return false
	var timing: String = str(labels["R_TIMING"]) % [0, _number_text(12.5, language, 2), 17]
	timing += "   " + str(labels["R_TARGET_TICKS"]) % _number_text(1000.0, language, 2)
	timing += "\n" + str(labels["R_SAFETY_DELAY"]) + "   " + str(labels["R_ADAPTIVE_TIMING"]) % [2, _number_text(500.0, language, 2)]
	if not _check((_application.get("_timing_label") as Label).text == timing, "Retain safety delay and backlog diagnostics while translating captions; actual=" + JSON.stringify((_application.get("_timing_label") as Label).text) + "; expected=" + JSON.stringify(timing)):
		return false
	if not _check(is_equal_approx(float(_application.get("_view_zoom")), 1.25), "Keep the requested actual canvas zoom"):
		return false
	var zoom: String = str(labels["R_ZOOM"]) % _number_text(125.0, language, 1)
	if not _check((_application.get("_zoom_label") as Label).text == zoom, "Refresh zoom text without changing the view"):
		return false
	return _check((_application.get("_placement_hint") as Label).visible and (_application.get("_placement_hint") as Label).text == str(labels["R_PLACE_RANDOM"]), "Refresh an active placement hint without resetting its owner")



func _number_bits(value: float) -> String:
	var bytes: PackedByteArray = PackedByteArray()
	bytes.resize(8)
	bytes.encode_double(0, value)
	bytes.reverse()
	return bytes.hex_encode()


func _number_text(value: float, language: String, precision: int) -> String:
	var bits: String = _number_bits(value)
	for sample: Dictionary in _expected[language]["number_samples"]:
		if str(sample["bits"]) == bits and int(sample["precision"]) == precision:
			return str(sample["text"])
	_check(false, "Missing independent UI numeric sample: " + language + "/" + bits + "/" + str(precision))
	return ""


func _number_cases(language: String) -> bool:
	var samples: Variant = _expected[language].get("number_samples")
	if not _check(samples is Array and samples.size() == 40, "Require independent finite and nonfinite samples for both precisions"):
		return false
	for sample: Dictionary in samples:
		var bits: String = str(sample["bits"])
		if not _check(bits.length() == 16 and int(sample["precision"]) in [1, 2], "Validate supplied numeric sample shape"):
			return false
		var bytes: PackedByteArray = PackedByteArray()
		for index: int in range(7, -1, -1):
			bytes.append(bits.substr(index * 2, 2).hex_to_int())
		var value: float = bytes.decode_double(0)
		if not _check(StatusLabels.format_number(value, language, int(sample["precision"])) == str(sample["text"]), "Compare formatter against an independent raw-bit golden: " + language + "/" + bits):
			return false
	return true


func _large_counters(language: String) -> bool:
	var previous: Dictionary = _application.get("_state")
	var presentation: Dictionary = previous.duplicate(true)
	presentation["world_time"] = 9223372036854775806
	presentation["tick"] = 9223372036854775805
	presentation["population"] = 2147483647
	_application.set("_state", presentation)
	_application.call("_update_labels")
	var text: String = (_application.get("_metrics_label") as Label).text
	_application.set("_state", previous)
	_application.call("_update_labels")
	var labels: Dictionary = _expected[language]
	return _check((str(labels["T_TIME"]) + "9223372036854775806") in text and (str(labels["R_TICK"]) + "9223372036854775805") in text and (str(labels["T_CURRENT_POPULATION"]) + "2147483647") in text, "Keep integer counter digits exact without floating conversion")


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
	print("BGR_MAIN_STATUS_BOUNDARY " + JSON.stringify({"status": "PASS" if passed else "FAIL", "failures": failures, "checks": _checks, "locales": 6}))
	quit(0 if passed else 1)
