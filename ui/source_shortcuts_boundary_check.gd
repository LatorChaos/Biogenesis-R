extends SceneTree

const Main = preload("res://ui/main.gd")
const LANGUAGES: Array[String] = ["en", "es", "ca", "de", "pt", "ru"]
const SAVE_KEYS: Array[int] = [KEY_S, KEY_G, KEY_D, KEY_S, KEY_S, KEY_S]
const BACKUP_KEYS: Array[int] = [KEY_B, KEY_R, KEY_G, KEY_K, KEY_B, KEY_B]

var _application: Control
var _simulation: BgrSimulation
var _path: String
var _backup: String
var _checks: int = 0
var _finished: bool = false


func _initialize() -> void:
	await process_frame
	var directory: String = OS.get_environment("BGR_SOURCE_SHORTCUTS_OUTPUT")
	if not _check(directory.is_absolute_path() and DirAccess.dir_exists_absolute(directory), "An existing private absolute output directory is required"):
		return
	_path = directory.path_join("shortcuts.bgw")
	_backup = directory.path_join("shortcuts@00000.bgw")
	if not _check(not FileAccess.file_exists(_path) and not FileAccess.file_exists(_backup) and not DirAccess.dir_exists_absolute(_path) and not DirAccess.dir_exists_absolute(_backup), "Owned scenario destinations must initially be absent"):
		return
	_simulation = BgrSimulation.new()
	_application = Main.new()
	_application.set("startup_seed_text", "827364")
	_application.set("startup_dimensions", Vector2i(96, 96))
	_application.set("startup_attempts", 0)
	if not _check(bool(_application.call("initialize_session", _simulation)), "Main must accept the actual native simulation"):
		return
	root.add_child(_application)
	_application.set_process(false)
	var initial: Dictionary = _simulation.snapshot()
	if not _check(bool(initial["ok"]) and int(initial["population"]) == 0 and int(initial["tick"]) == 0 and bool(initial["paused"]) and _application.call("applied_configuration") == PackedInt64Array([827364, 96, 96, 0]), "The owned zero-organism fixture must initialize through Main"):
		return
	if not _locale(0):
		return
	for running: bool in [false, true]:
		if running and not _pause_key(KEY_SPACE):
			return
		var before: Dictionary = _capture()
		_send(KEY_S)
		if not _check(str(_application.get("_file_operation")) == "world_save" and bool(_simulation.snapshot()["paused"]) and str(_application.get("_world_path")).is_empty(), "Unnamed Source Save must open the existing Save As owner"):
			return
		_application.call("_finish_file_operation")
		if not _check(_capture() == before and _absent(), "Cancelling unnamed Save must preserve activity, world, settings, and filename"):
			return
	var before_save_as: Dictionary = _capture()
	_send(KEY_S)
	if not _check(str(_application.get("_file_operation")) == "world_save", "Unnamed Save must remain available after cancellation"):
		return
	_application.call("_on_genotype_file_selected", _path.trim_suffix(".bgw"))
	before_save_as["world_path"] = _path
	if not _check(_capture() == before_save_as and FileAccess.file_exists(_path) and FileAccess.get_file_as_bytes(_path).size() > 0 and not FileAccess.file_exists(_backup), "Save As must write and adopt the actual file while restoring running activity"):
		return
	if not _pause_key(KEY_SPACE):
		return
	for index: int in range(LANGUAGES.size()):
		if not _locale(index) or not _hints(index):
			return
		for key: int in [KEY_P, KEY_P, KEY_SPACE, KEY_SPACE]:
			if not _pause_key(key):
				return
		if not _write_key(SAVE_KEYS[index], false) or not _pause_key(KEY_P) or not _write_key(SAVE_KEYS[index], false) or not _pause_key(KEY_P):
			return
		if index > 0 and SAVE_KEYS[index - 1] != SAVE_KEYS[index]:
			if not _stale_key(SAVE_KEYS[index - 1], SAVE_KEYS[index - 1] == BACKUP_KEYS[index]):
				return
		if not _write_key(BACKUP_KEYS[index], true):
			return
		for key: int in [KEY_P, SAVE_KEYS[index], BACKUP_KEYS[index]]:
			var legacy_control: bool = key == KEY_S or (LANGUAGES[index] == "es" and key == KEY_R)
			if not _ignored_variants(key, legacy_control):
				return
		if not _clear_destinations():
			return
		var field: LineEdit = _application.get("_seed_input") as LineEdit
		field.grab_focus()
		await process_frame
		if not _check(_application.get_viewport().gui_get_focus_owner() == field, "The real LineEdit must own control focus"):
			return
		var editing: Dictionary = _capture()
		var text: String = field.text
		for key: int in [KEY_P, SAVE_KEYS[index], BACKUP_KEYS[index]]:
			_send(key)
			if not _check(_capture() == editing and _absent() and field.text == text, "Plain shortcuts must leave text focus and model state unchanged: " + LANGUAGES[index]):
				return
		if not _write_key(KEY_S, false, 1):
			return
		_application.get_viewport().gui_release_focus()
		var before_reset: Dictionary = _capture()
		_send(KEY_R, 1 if LANGUAGES[index] == "es" else 0)
		if not _check(str(_application.get("_decision_operation")) == "reset", "The existing locale-aware Reset key must retain its decision owner"):
			return
		_application.call("_cancel_world_decision")
		if not _check(_capture() == before_reset, "Cancelling Reset must preserve the current world and settings"):
			return
	for name: String in ["_world_menu", "_actions_menu", "_world_tools_menu", "_genetics_menu", "_help_menu"]:
		var popup: PopupMenu = (_application.get(name) as MenuButton).get_popup()
		popup.popup()
		await process_frame
		if not _check(popup.visible, "The actual menu popup must own input") or not _blocked_keys(name):
			return
		popup.hide()
	var context: PopupMenu = _application.get("_world_context") as PopupMenu
	context.popup()
	await process_frame
	if not _check(context.visible, "The actual world context popup must own input") or not _blocked_keys("world context"):
		return
	context.hide()
	_application.call("_reset_world")
	if not _check(str(_application.get("_decision_operation")) == "reset", "The real world decision must open") or not _blocked_keys("world decision"):
		return
	_application.call("_cancel_world_decision")
	var before_modal: Dictionary = _capture()
	_application.call("_on_world_action", 13)
	if not _check(str(_application.get("_file_operation")) == "world_save_as", "The real Save As owner must open") or not _blocked_keys("Save As"):
		return
	_application.call("_finish_file_operation")
	if not _check(_capture() == before_modal, "Save As cancellation must preserve the named world"):
		return
	_application.call("_open_settings")
	await process_frame
	var settings: Window = _application.get("_settings_window") as Window
	if not _check(is_instance_valid(settings) and settings.visible, "The actual settings window must open") or not _blocked_keys("settings"):
		return
	settings.call("_cancel_draft")
	await process_frame
	_application.call("_on_help_action", 1)
	await process_frame
	var about: Window = _application.get("_about_window") as Window
	if not _check(is_instance_valid(about) and about.visible, "The actual About dialog must open without browser or network activity") or not _blocked_keys("About"):
		return
	about.call("close_about")
	await process_frame
	_application.get_viewport().gui_release_focus()
	if not _check(_capture() == before_modal, "Closing settings and About must preserve the world, settings, selection, and clipboard") or not _write_key(KEY_S, false, 1):
		return
	_finish(true)


func _locale(index: int) -> bool:
	var draft: BgrSettingsDraft = _simulation.settings_open()
	var controls: Dictionary = draft.snapshot()
	if not _check(bool(draft.replace_controls({"texts": controls["texts"], "locale": index, "automatic_backup": false, "hardware": controls["hardware"], "disable_framebuffer": controls["disable_framebuffer"]})["ok"]), "The actual locale draft must accept Source controls"):
		return false
	var before: Dictionary = _simulation.snapshot()
	return _check(bool(_simulation.settings_apply(draft, Callable(_application, "_settings_locale_changed"))["ok"]) and str(_simulation.settings_snapshot()["language"]) == LANGUAGES[index] and _simulation.snapshot() == before, "Live locale replacement must preserve the existing world: " + LANGUAGES[index])


func _hints(index: int) -> bool:
	var popup: PopupMenu = (_application.get("_world_menu") as MenuButton).get_popup()
	var save_name: String = OS.get_keycode_string(SAVE_KEYS[index])
	var backup_name: String = OS.get_keycode_string(BACKUP_KEYS[index])
	var reset_name: String = "Ctrl+R" if LANGUAGES[index] == "es" else "R"
	var help: String = (_application.get("_help_label") as Label).text
	return _check(("[%s / Ctrl+S]" % save_name) in popup.get_item_text(popup.get_item_index(12)) and ("[%s]" % backup_name) in popup.get_item_text(popup.get_item_index(14)) and "[P / Space]" in (_application.get("_pause_button") as Button).text and ("[%s]" % reset_name) in (_application.get("_reset_button") as Button).text and "P/Space" in help and ("%s/Ctrl+S" % save_name) in help and backup_name in help and reset_name in help, "Visible hints must identify all effective Source and retained keys: " + LANGUAGES[index])


func _capture() -> Dictionary:
	return {"world": _simulation.snapshot(), "settings": _simulation.settings_snapshot(), "world_path": str(_application.get("_world_path")), "operation": str(_application.get("_file_operation")), "decision": str(_application.get("_decision_operation"))}


func _pause_key(key: int) -> bool:
	var expected: Dictionary = _capture()
	expected["world"]["paused"] = not bool(expected["world"]["paused"])
	_send(key)
	return _check(_capture() == expected and "[P / Space]" in (_application.get("_pause_button") as Button).text, "One Pause/Resume press must toggle once without changing other model state")


func _write_key(key: int, backup: bool, modifier: int = 0) -> bool:
	if not _clear_destinations():
		return false
	var before: Dictionary = _capture()
	_send(key, modifier)
	var target: String = _backup if backup else _path
	var other: String = _path if backup else _backup
	return _check(FileAccess.file_exists(target) and FileAccess.get_file_as_bytes(target).size() > 0 and not FileAccess.file_exists(other) and _capture() == before and not (_application.get("_error_label") as Label).visible, "The effective key must reach the actual writer and preserve activity and model state")


func _stale_key(key: int, now_backup: bool) -> bool:
	if not _clear_destinations():
		return false
	var before: Dictionary = _capture()
	_send(key)
	return _check(not FileAccess.file_exists(_path) and FileAccess.file_exists(_backup) == now_backup and _capture() == before, "The previous Save key must stop saving while retaining its new Backup role")


func _ignored_variants(key: int, legacy_control: bool) -> bool:
	if not _clear_destinations():
		return false
	var before: Dictionary = _capture()
	for modifier: int in [1, 3, 4]:
		if modifier == 1 and legacy_control:
			continue
		_send(key, modifier)
		if key == KEY_R and modifier == 1:
			var reset: Dictionary = before.duplicate(true)
			reset["decision"] = "reset"
			if not _check(_capture() == reset and _absent(), "Modified R must retain the existing Reset route without saving or backing up"):
				return false
			_application.call("_cancel_world_decision")
			if not _check(_capture() == before and _absent(), "Cancelling modified-R Reset must preserve all world and settings state"):
				return false
		elif not _check(_capture() == before and _absent(), "Modified plain shortcut must not dispatch"):
			return false
	for repeat: bool in [true, false]:
		_send(key, 0, repeat, repeat)
		if not _check(_capture() == before and _absent(), "Echo and release must not dispatch"):
			return false
	return true


func _blocked_keys(owner: String) -> bool:
	if not _clear_destinations():
		return false
	var before: Dictionary = _capture()
	for key: int in [KEY_P, KEY_S, KEY_B]:
		_send(key)
		if not _check(_capture() == before and _absent(), "Modal or popup input must retain ownership: " + owner):
			return false
	return true


func _send(key: int, modifier: int = 0, pressed: bool = true, echo: bool = false) -> void:
	var event: InputEventKey = InputEventKey.new()
	event.pressed = pressed
	event.echo = echo
	event.keycode = key
	event.physical_keycode = key
	event.ctrl_pressed = modifier == 1
	event.alt_pressed = modifier == 2
	event.meta_pressed = modifier == 3
	event.shift_pressed = modifier == 4
	_application.call("_input", event)


func _absent() -> bool:
	return not FileAccess.file_exists(_path) and not FileAccess.file_exists(_backup)


func _clear_destinations() -> bool:
	var valid: bool = true
	for path: String in [_path, _backup]:
		valid = valid and not DirAccess.dir_exists_absolute(path)
		if FileAccess.file_exists(path):
			valid = valid and DirAccess.remove_absolute(path) == OK
	return _check(valid and _absent(), "Only the two owned scenario files may be removed")


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
	var failures: Array[String] = []
	if not passed:
		failures.append(message)
	print("BGR_SOURCE_SHORTCUTS_BOUNDARY " + JSON.stringify({"status": "PASS" if passed else "FAIL", "checks": _checks, "failures": failures, "locales": 6}))
	quit(0 if passed else 1)
