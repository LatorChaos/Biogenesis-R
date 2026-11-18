extends SceneTree

const Main = preload("res://ui/main.gd")
const Labels = preload("res://ui/main_labels.gd")
const LANGUAGES: Array[String] = ["en", "es", "ca", "de", "pt", "ru"]

var _application: Control
var _simulation: BgrSimulation
var _expected: Dictionary
var _status_expected: Dictionary
var _checks: int = 0
var _finished: bool = false


func _initialize() -> void:
	await process_frame
	var expected_path: String = OS.get_environment("BGR_PAUSE_EXPECTED")
	if not _check(expected_path.is_absolute_path() and FileAccess.file_exists(expected_path), "Require the independent Source48 fixture"):
		return
	_expected = JSON.parse_string(FileAccess.get_file_as_string(expected_path))
	if not _check(_expected.size() == 6, "Source48 must cover six locales"):
		return
	var status_values: Variant = JSON.parse_string(FileAccess.get_file_as_string("res://ui/main_status_expected.json"))
	if not _check(status_values is Dictionary and status_values.size() == 6, "Require independent status goldens for six locales"):
		return
	_status_expected = status_values
	for language: String in LANGUAGES:
		if not _check(_status_expected.get(language) is Dictionary and _status_expected[language].get("R_STOPPED_ERROR") is String and _status_expected[language].get("R_INITIALIZATION_ERROR") is String, "Require independent failure mode labels: " + language):
			return
	_simulation = BgrSimulation.new()
	_application = Main.new()
	_application.set("startup_seed_text", "827364")
	_application.set("startup_dimensions", Vector2i(96, 96))
	_application.set("startup_attempts", 0)
	if not _check(bool(_application.call("initialize_session", _simulation)), "Use the actual Rust session"):
		return
	root.add_child(_application)
	_application.set_process(false)
	await process_frame
	if not _check(int(_simulation.snapshot()["population"]) == 0, "The private world must have zero organisms"):
		return
	for index: int in range(LANGUAGES.size()):
		if not _locale(index):
			return
		for running: bool in [false, true]:
			if not _assign(running) or not _presentation(index, running) or not _assign(running):
				return
			var row: Dictionary = _expected[LANGUAGES[index]]
			var active: int = OS.find_keycode_from_string(str(row["T_PAUSE_MNEMONIC" if running else "T_RESUME_MNEMONIC"]))
			var inactive: int = OS.find_keycode_from_string(str(row["T_RESUME_MNEMONIC" if running else "T_PAUSE_MNEMONIC"]))
			for route: String in ["toolbar", "menu", "P", "Space", "Alt", "popup"]:
				if not _assign(running) or not _toggle(route, active):
					return
			if not _assign(running):
				return
			for event: InputEventKey in [_key(inactive, true), _key(active, true, true), _key(active, true, false, true), _key(active, true, false, false, true), _key(active, true, false, false, false, false), _key(active, true, false, false, false, true, true)]:
				var before: Dictionary = _capture()
				_application.call("_input", event)
				if not _check(_capture() == before, "Inactive, modified, release, and echo action mnemonics must preserve state"):
					return
			var field: LineEdit = _application.get("_seed_input") as LineEdit
			field.grab_focus()
			await process_frame
			var before_text: Dictionary = _capture()
			_application.call("_input", _key(active, true))
			if not _check(_application.get_viewport().gui_get_focus_owner() == field and _capture() == before_text, "Text focus must own the mnemonic"):
				return
			_application.get_viewport().gui_release_focus()
			if not _locale((index + 1) % 6) or not _presentation((index + 1) % 6, running) or not _locale(index):
				return
	if not _check(Labels.message("T_RESUME", "unsupported") == str(_expected["en"]["T_RESUME"]), "Unknown locale presentation must use English"):
		return
	if not _assign(false):
		return
	_application.set("_runtime_failed", true)
	_application.call("_update_labels")
	_application.call("_settings_locale_changed", {"language": "de"})
	var popup: PopupMenu = (_application.get("_world_menu") as MenuButton).get_popup()
	if not _check((_application.get("_mode_label") as Label).text == str(_status_expected["de"]["R_STOPPED_ERROR"]) and (_application.get("_pause_button") as Button).disabled and popup.is_item_disabled(popup.get_item_index(16)), "Runtime errors must retain priority and disable both action widgets"):
		return
	var failed: Dictionary = _capture()
	_application.call("_on_world_action", 16)
	if not _check(_capture() == failed and not popup.is_item_disabled(popup.get_item_index(15)), "Failed pause action must preserve recovery access"):
		return
	_application.set("_runtime_failed", false)
	_application.set("_state", {})
	_application.call("_show_error", "owned initialization failure")
	_application.call("_settings_locale_changed", {"language": "ru"})
	if not _check((_application.get("_mode_label") as Label).text == str(_status_expected["ru"]["R_INITIALIZATION_ERROR"]) and (_application.get("_error_label") as Label).visible, "Locale refresh must preserve initialization failure"):
		return
	_finish(true)


func _locale(index: int) -> bool:
	var draft: BgrSettingsDraft = _simulation.settings_open()
	var controls: Dictionary = draft.snapshot()
	if not _check(bool(draft.replace_controls({"texts": controls["texts"], "locale": index, "automatic_backup": false, "hardware": controls["hardware"], "disable_framebuffer": controls["disable_framebuffer"]})["ok"]), "Edit the real locale draft"):
		return false
	var before: Dictionary = _simulation.snapshot()
	return _check(bool(_simulation.settings_apply(draft, Callable(_application, "_settings_locale_changed"))["ok"]) and _simulation.snapshot() == before, "Locale callback must preserve world activity and state")


func _assign(running: bool) -> bool:
	var expected: Dictionary = _simulation.snapshot()
	expected["paused"] = not running
	return _check(bool(_application.call("_accept_result", _simulation.apply_commands(PackedInt32Array([1 if running else 0])))) and _simulation.snapshot() == expected, "Repeated explicit state assignment must be idempotent")


func _presentation(index: int, running: bool) -> bool:
	var values: Dictionary = _expected[LANGUAGES[index]]
	var prefix: String = "T_PAUSE" if running else "T_RESUME"
	var button: Button = _application.get("_pause_button") as Button
	var popup: PopupMenu = (_application.get("_world_menu") as MenuButton).get_popup()
	var item: int = popup.get_item_index(16)
	if not _check(button.text == str(values[prefix]) + " [P / Space]" and button.tooltip_text == str(values[prefix + "_PROCESS"]) and not button.disabled, "Toolbar caption and description must match Source and current state"):
		return false
	if not _check(button.icon != null and button.icon.get_width() == 16 and button.icon.get_height() == 16 and button.icon.resource_path.ends_with("pause.svg" if running else "resume.svg"), "Toolbar must use the corresponding authored glyph"):
		return false
	if not _check(item >= 0 and popup.get_item_text(item) == str(values[prefix]) and popup.get_item_tooltip(item) == str(values[prefix + "_PROCESS"]) and popup.get_item_icon(item) == null and not popup.is_item_disabled(item), "World menu must project the same action without an icon"):
		return false
	return _check((_application.get("_mode_label") as Label).text == str(values["T_GAME_RESUMED" if running else "T_GAME_PAUSED"]), "Normal status must match Source language and current state")


func _toggle(route: String, active: int) -> bool:
	var expected: Dictionary = _capture()
	expected["world"]["paused"] = not bool(expected["world"]["paused"])
	match route:
		"toolbar": (_application.get("_pause_button") as Button).pressed.emit()
		"menu": (_application.get("_world_menu") as MenuButton).get_popup().id_pressed.emit(16)
		"P": _application.call("_input", _key(KEY_P))
		"Space": _application.call("_input", _key(KEY_SPACE))
		"Alt": _application.call("_input", _key(active, true))
		"popup":
			var popup: PopupMenu = (_application.get("_world_menu") as MenuButton).get_popup()
			popup.popup()
			if not _check(popup.visible, "Open the real popup before its owned signal"):
				return false
			popup.window_input.emit(_key(active))
			if not _check(not popup.visible, "Popup-owned action must close its popup"):
				return false
	return _check(_capture() == expected, "One route must toggle once without changing unrelated world or settings state: " + route)


func _capture() -> Dictionary:
	return {"world": _simulation.snapshot(), "settings": _simulation.settings_snapshot(), "operation": str(_application.get("_file_operation")), "decision": str(_application.get("_decision_operation"))}


func _key(code: int, alt: bool = false, shift: bool = false, ctrl: bool = false, meta: bool = false, pressed: bool = true, echo: bool = false) -> InputEventKey:
	var event: InputEventKey = InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.alt_pressed = alt
	event.shift_pressed = shift
	event.ctrl_pressed = ctrl
	event.meta_pressed = meta
	event.pressed = pressed
	event.echo = echo
	return event


func _check(value: bool, message: String) -> bool:
	_checks += 1
	if not value:
		_finish(false, message)
	return value


func _finish(passed: bool, message: String = "") -> void:
	if _finished:
		return
	_finished = true
	if is_instance_valid(_application):
		_application.call("shutdown")
		_application.queue_free()
	print("BGR_PAUSE_ACTION_BOUNDARY " + JSON.stringify({"status": "PASS" if passed else "FAIL", "checks": _checks, "failures": [] if passed else [message], "locales": 6}))
	quit(0 if passed else 1)
