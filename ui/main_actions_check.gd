extends "res://ui/source_shortcuts_boundary_check.gd"

const MENU_CONTRACT: Dictionary = {
	"_world_menu": ["T_GAME", [17, 16, 11, 12, 13, 14, 15, 18]],
	"_actions_menu": ["T_GLOBAL_ACTIONS", [6, 7, 8, 9, 4, 5]],
	"_world_tools_menu": ["T_WORLD", [19, 20, 21, 10]]
}
const ITEM_LABELS: Dictionary = {
	4: ["T_KILL_ALL", "T_KILL_ALL_ORGANISMS"],
	5: ["T_DISPERSE_ALL", "T_DISPERSE_ALL_DEAD_ORGANISMS"],
	6: ["T_INCREASE_CO2", "T_INCREASE_CO2"],
	7: ["T_DECREASE_CO2", "T_DECREASE_CO2"],
	8: ["T_INCREASE_CH4", "T_INCREASE_CH4"],
	9: ["T_DECREASE_CH4", "T_DECREASE_CH4"],
	10: ["T_ABORT_TRACKING", "T_ABORT_TRACKING"],
	11: ["T_OPEN", "T_OPEN_WORLD"],
	12: ["T_SAVE", "T_SAVE_WORLD"],
	13: ["T_SAVE_AS", "T_SAVE_WORLD_AS"],
	14: ["T_BACKUP", "T_BACKUP_WORLD"],
	15: ["R_MANAGED_RECOVER", "R_MANAGED_RECOVER_PROCESS"],
	16: ["T_RESUME", "T_RESUME_PROCESS"],
	17: ["T_NEW", "T_NEW_WORLD"],
	18: ["T_QUIT", "T_QUIT_PROGRAM"],
	19: ["T_STATISTICS", "T_WORLD_STATISTICS"],
	20: ["T_GENETIC_LABORATORY", "T_GENETIC_LABORATORY"],
	21: ["T_PARAMETERS", "T_EDIT_PARAMETERS"]
}

const HEADING_LABELS: Array[String] = ["T_GAME_MNEMONIC", "T_GLOBAL_ACTIONS_MNEMONIC", "T_WORLD_MNEMONIC", "T_NETWORK_MNEMONIC", "T_HELP_MNEMONIC"]
const HEADING_POPUPS: Array[String] = ["_world_menu", "_actions_menu", "_world_tools_menu", "_network_menu", "_help_menu"]

var _labels: Dictionary


func _initialize() -> void:
	await process_frame
	var expected: String = OS.get_environment("BGR_MAIN_ACTIONS_EXPECTED")
	var directory: String = OS.get_environment("BGR_MAIN_ACTIONS_OUTPUT")
	if not _check(expected.is_absolute_path() and FileAccess.file_exists(expected) and directory.is_absolute_path() and DirAccess.dir_exists_absolute(directory), "Require independent labels and an existing private output directory"):
		return
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(expected))
	if not _check(parsed is Dictionary and parsed.size() == 6, "Independent labels must cover six locales"):
		return
	_labels = parsed
	for language: String in LANGUAGES:
		if not _check(_labels.get(language) is Dictionary, "Missing independent locale labels: " + language):
			return
		for pair: Array in ITEM_LABELS.values():
			for key: String in pair:
				if not _check(_labels[language].get(key) is String, "Missing independent action label: " + language + "/" + key):
					return
	_path = directory.path_join("main-actions.bgw")
	_backup = directory.path_join("main-actions@00000.bgw")
	if not _check(_absent() and not DirAccess.dir_exists_absolute(_path) and not DirAccess.dir_exists_absolute(_backup), "Preserve existing scenario destinations"):
		return
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
	if not _check(int(_simulation.snapshot()["population"]) == 0 and int(_simulation.snapshot()["tick"]) == 0 and bool(_simulation.snapshot()["paused"]), "Initialize the owned paused empty world"):
		return
	for index: int in range(LANGUAGES.size()):
		if not _locale(index) or not _presentation(index):
			return
		var canvas: Control = _application.get("_canvas") as Control
		canvas.call("zoom_at", 1.25, canvas.size * 0.5)
		if not _save_routes(index) or not _invalid_events(index):
			return
		var field: LineEdit = _application.get("_seed_input") as LineEdit
		field.grab_focus()
		await process_frame
		if not _text_guard(index):
			return
		_application.get_viewport().gui_release_focus()
		_application.call("_open_settings")
		await process_frame
		var settings: Window = _application.get("_settings_window") as Window
		if not _check(is_instance_valid(settings) and settings.visible, "Open the actual Settings modal") or not _blocked_actions(index):
			return
		settings.call("_cancel_draft")
		await process_frame
		_application.get_window().grab_focus()
		_application.get_viewport().gui_release_focus()
		await process_frame
		if not _check(not bool(_application.call("_settings_open")), "Cancel Settings through its existing owner"):
			return
		if not _new_game_popup() or not await _witness_cases(index):
			return
	_finish(true)


func _presentation(index: int) -> bool:
	var values: Dictionary = _labels[LANGUAGES[index]]
	for property: String in MENU_CONTRACT:
		var contract: Array = MENU_CONTRACT[property]
		var menu: MenuButton = _application.get(property) as MenuButton
		var popup: PopupMenu = menu.get_popup()
		var ids: Array[int] = []
		if not _check(menu.text == str(values[contract[0]]), "Source menu heading differs: " + property):
			return false
		for item: int in range(popup.item_count):
			if popup.is_item_separator(item):
				continue
			var id: int = popup.get_item_id(item)
			ids.append(id)
			if not _check(ITEM_LABELS.has(id), "Unexpected action ID in adopted menu"):
				return false
			var pair: Array = ITEM_LABELS[id]
			var caption: String = str(values[pair[0]])
			if id == 12:
				caption += " [%s / Ctrl+S]" % OS.get_keycode_string(SAVE_KEYS[index])
			elif id == 14:
				caption += " [%s]" % OS.get_keycode_string(BACKUP_KEYS[index])
			if not _check(popup.get_item_text(item) == caption and popup.get_item_tooltip(item) == str(values[pair[1]]), "Independent action caption or description differs: " + str(id)):
				return false
		if not _check(ids == contract[1], "Stable menu groups or command IDs changed: " + property):
			return false
	for entry: Array in [["SaveWorldButton", 12], ["BackupWorldButton", 14], ["IncreaseCO2Button", 6], ["DecreaseCO2Button", 7], ["AbortTrackingButton", 10], ["CreateWorldButton", 17], ["SettingsButton", 21], ["StatisticsButton", 19], ["QuitButton", 18]]:
		var button: Button = _application.find_child(str(entry[0]), true, false) as Button
		var pair: Array = ITEM_LABELS[int(entry[1])]
		if not _check(is_instance_valid(button) and button.tooltip_text == str(values[pair[1]]) and button.accessibility_name == str(values[pair[0]]), "Quick action description or accessible name differs: " + str(entry[0])):
			return false
	return true


func _save_routes(index: int) -> bool:
	if not _clear_destinations():
		return false
	var before: Dictionary = _capture()
	_send(SAVE_KEYS[index])
	if str(_application.get("_world_path")).is_empty():
		if not _check(str(_application.get("_file_operation")) == "world_save", "Unnamed explicit Save must reach Save As"):
			return false
		_application.call("_on_genotype_file_selected", _path)
		before["world_path"] = _path
	if not _check(FileAccess.file_exists(_path) and _capture() == before, "Explicit locale Save must write without altering the model"):
		return false
	if index != 2:
		if not _check(DirAccess.remove_absolute(_path) == OK, "Remove only the owned Save fixture"):
			return false
		_send(SAVE_KEYS[index], 2)
		if not _check(FileAccess.file_exists(_path) and _capture() == before, "Nonconflicting Source toolbar Alt Save must write once"):
			return false
	_send(BACKUP_KEYS[index])
	if not _check(FileAccess.file_exists(_backup) and _capture() == before, "Explicit locale Backup must reach the existing writer"):
		return false
	if index != 1:
		if not _check(DirAccess.remove_absolute(_backup) == OK, "Remove only the owned Backup fixture"):
			return false
		_send(BACKUP_KEYS[index], 2)
		if not _check(FileAccess.file_exists(_backup) and _capture() == before, "Nonconflicting Source toolbar Alt Backup must write once"):
			return false
	var field: LineEdit = _application.get("_seed_input") as LineEdit
	field.grab_focus()
	if not _check(DirAccess.remove_absolute(_path) == OK, "Prepare the owned Ctrl+S destination"):
		return false
	_send(KEY_S, 1)
	if not _check(FileAccess.file_exists(_path) and _capture() == before, "Qualified Ctrl+S must remain available from Main text focus"):
		return false
	_application.get_viewport().gui_release_focus()
	return _clear_destinations()


func _invalid_events(index: int) -> bool:
	var before: Dictionary = _capture()
	var values: Dictionary = _labels[LANGUAGES[index]]
	var codes: Array[int] = [KEY_N, KEY_R, KEY_F, KEY_Q, KEY_P, SAVE_KEYS[index], BACKUP_KEYS[index]]
	for heading: String in HEADING_LABELS:
		codes.append(OS.find_keycode_from_string(str(values[heading])))
	for code: int in codes:
		for event: InputEventKey in [_event(code, true, true), _event(code, false, true), _event(code, false, false, true), _event(code, true, false, true), _event(code, true, false, false, false), _event(code, true, false, false, true, true)]:
			_application.call("_input", event)
			if not _check(_capture() == before and _closed() and _absent(), "Modified, echo, and release gestures must not leak into legacy actions"):
				return false
	return true


func _text_guard(index: int) -> bool:
	var field: LineEdit = _application.get("_seed_input") as LineEdit
	if not _check(_application.get_viewport().gui_get_focus_owner() == field, "The actual Main LineEdit must own focus"):
		return false
	var before: Dictionary = _capture()
	var values: Dictionary = _labels[LANGUAGES[index]]
	for code: int in [KEY_P, SAVE_KEYS[index], BACKUP_KEYS[index]]:
		_application.call("_input", _event(code))
		if not _check(_capture() == before and _closed() and _absent(), "Plain aliases must respect Main text ownership"):
			return false
	var heading_codes: Array[int] = []
	for heading: String in HEADING_LABELS:
		heading_codes.append(OS.find_keycode_from_string(str(values[heading])))
	for code: int in [SAVE_KEYS[index], BACKUP_KEYS[index], KEY_I, KEY_D]:
		if code in heading_codes:
			continue
		_application.call("_input", _event(code, true))
		if not _check(_capture() == before and _closed() and _absent(), "Toolbar mnemonics must respect Main text ownership"):
			return false
	for heading_index: int in range(HEADING_LABELS.size()):
		if not _check(_closed(), "Heading gestures must begin with all Main popups closed"):
			return false
		field.grab_focus()
		if not _check(_application.get_viewport().gui_get_focus_owner() == field, "The actual Main LineEdit must retain editing ownership"):
			return false
		var expected_popup: String = HEADING_POPUPS[heading_index]
		# Source Help wins the shared Actions mnemonic in Spanish and Catalan
		if HEADING_LABELS[heading_index] == "T_GLOBAL_ACTIONS_MNEMONIC" and LANGUAGES[index] in ["es", "ca"]:
			expected_popup = "_help_menu"
		_send(heading_codes[heading_index], 2)
		if not _check(_popup(expected_popup).visible and _capture() == before and _absent(), "Main menu headings must remain available while editing"):
			return false
		_popup(expected_popup).hide()
		if not _check(_closed() and _capture() == before and _absent(), "Closing a heading popup must preserve the editing scenario"):
			return false
	field.grab_focus()
	return true


func _blocked_actions(index: int) -> bool:
	var before: Dictionary = _capture()
	var values: Dictionary = _labels[LANGUAGES[index]]
	for code: int in [KEY_P, SAVE_KEYS[index], BACKUP_KEYS[index], KEY_N, KEY_I, KEY_D, OS.find_keycode_from_string(str(values["T_WORLD_MNEMONIC"]))]:
		for alt: bool in [false, true]:
			_application.call("_input", _event(code, alt))
			if not _check(_capture() == before and _closed() and _absent(), "Settings modal must retain ownership of Main action keys"):
				return false
	_send(KEY_S, 1)
	return _check(_capture() == before and _closed() and _absent(), "Settings must also block the qualified Ctrl+S alias")


func _new_game_popup() -> bool:
	var popup: PopupMenu = _popup("_world_menu")
	popup.popup()
	var before: Dictionary = _capture()
	popup.window_input.emit(_event(KEY_N))
	if not _check(not popup.visible and str(_application.get("_decision_operation")) == "create" and _simulation.snapshot() == before["world"], "Game popup New must stay reachable despite reserved Network heading N"):
		return false
	_application.call("_cancel_world_decision")
	return _check(_capture() == before, "Cancel New Game must preserve model and activity")


func _witness_cases(index: int) -> bool:
	var language: String = LANGUAGES[index]
	var values: Dictionary = _labels[language]
	var before: Dictionary = _capture()
	_send(OS.find_keycode_from_string(str(values["T_NETWORK_MNEMONIC"])), 2)
	var network_popup: PopupMenu = _popup("_network_menu")
	if not _check(network_popup.visible and _capture() == before and _absent(), "Network heading must open its actual popup without changing model or files"):
		return false
	network_popup.hide()
	if not _check(_closed() and _capture() == before and _absent(), "Closing the Network heading must retain the scenario"):
		return false
	if language == "ca":
		if not _check(float(before["world"]["carbon_dioxide"]) >= 1000.0, "Catalan collision fixture needs removable CO2"):
			return false
		_send(KEY_D, 2)
		before["world"]["carbon_dioxide"] = float(before["world"]["carbon_dioxide"]) - 1000.0
		if not _check(_capture() == before and _closed() and _absent(), "Source Catalan Alt+D must decrease CO2 instead of saving"):
			return false
	if language == "de":
		var managers: Dictionary = _application.get("_network_managers")
		if not _check(managers.is_empty() and not bool(_application.call("_network_open")) and str(values["T_MANAGE_CONNECTIONS_MNEMONIC"]) == "V", "German Connections witness requires its declared key and an unowned manager slot"):
			return false
		_send(KEY_V, 2)
		await process_frame
		managers = _application.get("_network_managers")
		if not _check(managers.size() == 1 and _closed() and _capture() == before and _absent(), "German Alt+V must open one Connections manager without decreasing CO2 or changing files"):
			return false
		var reference: WeakRef = managers.values()[0] as WeakRef
		if not _check(reference != null, "Main must retain the Connections manager through its existing weak owner"):
			return false
		var manager: Window = reference.get_ref() as Window
		if not _check(is_instance_valid(manager) and manager.get_parent() == _application and managers.has(manager.get_instance_id()) and manager.visible and int(manager.get("initial_mode")) == 1 and not bool(_application.call("_network_open")), "Connections must use the actual visible mode1 manager, not the mode0 configuration owner"):
			return false
		var tabs: TabContainer = manager.get("_tabs") as TabContainer
		if not _check(tabs != null and tabs.current_tab == 1, "Connections manager must display its Connections tab"):
			return false
		manager.call("close_window")
		await manager.tree_exited
		_application.get_window().grab_focus()
		_application.get_viewport().gui_release_focus()
		await process_frame
		managers = _application.get("_network_managers")
		if not _check(managers.is_empty() and not bool(_application.call("_network_open")) and _closed() and _capture() == before and _absent(), "Closing Connections must remove its owner and preserve model, activity, and files"):
			return false
	for repetition: int in range(2):
		root.push_input(_event(OS.find_keycode_from_string(str(values["T_HELP_MNEMONIC"])), true))
		await process_frame
		if not _check(_popup("_help_menu").visible and _focused_id(_popup("_help_menu")) == 0 and not _popup("_actions_menu").visible and _capture() == before, "Help heading must select Manual first and retain its owner"):
			return false
	_popup("_help_menu").hide()
	if language == "en":
		_send(KEY_W, 2)
		var tools: PopupMenu = _popup("_world_tools_menu")
		if not _check(tools.visible and _capture() == before, "English Alt+W must open World without changing the model"):
			return false
		tools.window_input.emit(_event(KEY_P))
		await process_frame
		var settings: Window = _application.get("_settings_window") as Window
		if not _check(is_instance_valid(settings) and settings.visible and not tools.visible and _capture() == before, "World popup P must open Parameters without toggling pause"):
			return false
		settings.call("_cancel_draft")
		await process_frame
	if language in ["de", "pt"]:
		_send(KEY_A, 2)
		var actions: PopupMenu = _popup("_actions_menu")
		if not _check(actions.visible and _focused_id(actions) == 6 and _capture() == before, "Source Actions heading initially selects increase CO2"):
			return false
		var code: int = KEY_E if language == "de" else KEY_I
		actions.window_input.emit(_event(code))
		if not _check(actions.visible and _focused_id(actions) == 9 and _capture() == before, "First duplicate key selects decrease CH4 without dispatch"):
			return false
		actions.window_input.emit(_event(code))
		if not _check(actions.visible and _focused_id(actions) == 6 and _capture() == before, "Second duplicate key selects increase CO2 without dispatch"):
			return false
		var selected_id: int = _focused_id(actions)
		actions.hide()
		actions.id_pressed.emit(selected_id)
		await process_frame
		before["world"]["carbon_dioxide"] = float(before["world"]["carbon_dioxide"]) + 1000.0
		if not _check(not actions.visible and _capture() == before, "Focused-item callback must execute the selected CO2 action exactly once; native Enter is checked separately"):
			return false
	return true


func _event(code: int, alt: bool = false, shift: bool = false, meta: bool = false, pressed: bool = true, echo: bool = false) -> InputEventKey:
	var event: InputEventKey = InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	event.echo = echo
	event.alt_pressed = alt
	event.shift_pressed = shift
	event.meta_pressed = meta
	return event


func _popup(property: String) -> PopupMenu:
	return (_application.get(property) as MenuButton).get_popup()


func _focused_id(popup: PopupMenu) -> int:
	var item: int = popup.get_focused_item()
	return popup.get_item_id(item) if item >= 0 else -1


func _closed() -> bool:
	for property: String in ["_world_menu", "_actions_menu", "_world_tools_menu", "_network_menu", "_genetics_menu", "_help_menu"]:
		if _popup(property).visible:
			return false
	return true


func _capture() -> Dictionary:
	var result: Dictionary = super._capture()
	var canvas: Control = _application.get("_canvas") as Control
	result["view"] = {"zoom": canvas.get("_zoom"), "offset": canvas.get("_offset"), "fit": canvas.get("_fit_enabled")}
	return result


func _finish(passed: bool, message: String = "") -> void:
	if _finished:
		return
	_finished = true
	if is_instance_valid(_application):
		_application.call("shutdown")
		_application.queue_free()
	print("BGR_MAIN_ACTIONS_BOUNDARY " + JSON.stringify({"status": "PASS" if passed else "FAIL", "checks": _checks, "failures": [] if passed else [message], "locales": 6, "native_input": false}))
	quit(0 if passed else 1)
