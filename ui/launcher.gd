extends Control

const MainLabels = preload("res://ui/main_labels.gd")
const Main = preload("res://ui/main.gd")
const NEW_PROFILE_CO2: int = 125000
const RendererDialog = preload("res://ui/renderer_dialog.gd")
const StartupArguments = preload("res://ui/startup_arguments.gd")
const FileFilterLabels = preload("res://ui/file_filter_labels.gd")

var _simulation_window: Window
var _main: Main
var _simulation: BgrSimulation
var _start_stop: Button
var _status: Label
var _seed_text: String = ""
var _exit_when_closed: bool = false
var _geometry_notice: String = ""
var _geometry_notice_key: String = ""
var _import_button: Button
var _import_dialog: FileDialog
var _import_result: AcceptDialog
var _quit_button: Button
var _language: String = "en"
var _status_key: String = "R_LAUNCHER_STOPPED"
var _import_result_state: Dictionary = {}
var _reset_button: Button
var _restore_button: Button
var _preference_confirm: ConfirmationDialog
var _preference_result: AcceptDialog
var _preference_action: StringName = &""
var _preference_action_language: String = "en"
var _preference_focus: Button
var _reset_backup_available: bool = false
var _renderer_dialog: RendererDialog
var _renderer_button: Button
var _renderer_notice: Label
var _launcher_only: bool = false
var _resume_managed: bool = false
var _new_profile_defaults: bool = false


func _ready() -> void:
	get_tree().auto_accept_quit = false
	get_window().min_size = Vector2i(360, 180)
	get_window().wrap_controls = true
	get_window().size = Vector2i(420, 220)
	get_window().close_requested.connect(_quit_launcher)
	_simulation = BgrSimulation.new()
	var preferences: Dictionary = _simulation.preferences_initialize()
	_new_profile_defaults = bool(preferences["ok"]) and str(preferences["status"]) == "missing"
	_language = str(preferences["settings"]["language"])
	_reset_backup_available = bool(preferences["reset_backup_available"])
	_build_interface()
	if not str(preferences["error"]).is_empty():
		_status.text = _preference_error_status(preferences)
	var user_arguments: PackedStringArray = OS.get_cmdline_user_args()
	var safe_graphics: bool = user_arguments.has("--safe-graphics")
	while user_arguments.has("--safe-graphics"):
		user_arguments.remove_at(user_arguments.find("--safe-graphics"))
	var endurance_options: Dictionary = Main.EnduranceMetrics.options(user_arguments)
	if not str(endurance_options["error"]).is_empty():
		printerr("BGR_ENDURANCE_FAILURE " + str(endurance_options["error"]))
	user_arguments = Main.EnduranceMetrics.gameplay_arguments(user_arguments)
	var startup: Dictionary = StartupArguments.parse(user_arguments)
	_seed_text = str(startup["seed_text"])
	if not bool(startup["valid"]):
		printerr(StartupArguments.USAGE)
	_launcher_only = bool(startup["launcher_only"])
	_resume_managed = bool(startup["resume_managed"])
	var renderer_pending: bool = _renderer_dialog.begin_startup(safe_graphics)
	_update_launcher_actions()
	if not renderer_pending and not _launcher_only:
		_start_simulation.call_deferred()


func _build_interface() -> void:
	var margin: MarginContainer = MarginContainer.new()
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for side: String in ["left", "top", "right", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 20)
	add_child(margin)
	margin.minimum_size_changed.connect(func() -> void:
		custom_minimum_size = margin.get_combined_minimum_size()
	)
	var column: VBoxContainer = VBoxContainer.new()
	column.add_theme_constant_override("separation", 14)
	margin.add_child(column)
	var title: Label = Label.new()
	title.text = "Biogenesis-R"
	title.add_theme_font_size_override("font_size", 24)
	column.add_child(title)
	_status = Label.new()
	_status.name = "LauncherStatus"
	_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	column.add_child(_status)
	var actions: HFlowContainer = HFlowContainer.new()
	column.add_child(actions)
	_start_stop = Button.new()
	_start_stop.name = "StartStopButton"
	_start_stop.pressed.connect(_toggle_simulation)
	actions.add_child(_start_stop)
	_quit_button = Button.new()
	_quit_button.name = "LauncherQuitButton"
	_quit_button.pressed.connect(_quit_launcher)
	actions.add_child(_quit_button)
	_import_button = Button.new()
	_import_button.name = "ImportPreferencesButton"
	_import_button.pressed.connect(_choose_preferences)
	actions.add_child(_import_button)
	_reset_button = Button.new()
	_reset_button.name = "ResetPreferencesButton"
	_reset_button.pressed.connect(func() -> void: _choose_preference_action(&"reset"))
	actions.add_child(_reset_button)
	_restore_button = Button.new()
	_restore_button.name = "RestorePreferencesButton"
	_restore_button.pressed.connect(func() -> void: _choose_preference_action(&"restore"))
	actions.add_child(_restore_button)
	_renderer_button = Button.new()
	_renderer_button.name = "RendererSettingsButton"
	_renderer_button.pressed.connect(func() -> void: _renderer_dialog.open_settings())
	actions.add_child(_renderer_button)
	_renderer_notice = Label.new()
	_renderer_notice.name = "RendererLauncherNotice"
	_renderer_notice.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_renderer_notice.custom_minimum_size.x = 320
	_renderer_notice.custom_maximum_size.x = 380
	column.add_child(_renderer_notice)
	_import_dialog = FileDialog.new()
	_import_dialog.hide()
	_import_dialog.force_native = true
	FileFilterLabels.fit_filter_labels(_import_dialog)
	_import_dialog.name = "LegacyPreferencesImportDialog"
	_import_dialog.access = FileDialog.ACCESS_FILESYSTEM
	_import_dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE
	_import_dialog.file_selected.connect(_import_preferences)
	_import_dialog.canceled.connect(_cancel_import)
	add_child(_import_dialog)
	_import_result = AcceptDialog.new()
	_import_result.force_native = true
	_import_result.name = "LegacyPreferencesImportResult"
	_import_result.dialog_autowrap = true
	_import_result.confirmed.connect(_close_import_result)
	_import_result.canceled.connect(_close_import_result)
	add_child(_import_result)
	_preference_confirm = ConfirmationDialog.new()
	_preference_confirm.name = "PreferenceActionConfirmation"
	_preference_confirm.force_native = true
	_preference_confirm.dialog_autowrap = true
	_preference_confirm.confirmed.connect(_apply_preference_action)
	_preference_confirm.canceled.connect(_cancel_preference_action)
	add_child(_preference_confirm)
	_preference_result = AcceptDialog.new()
	_preference_result.name = "PreferenceActionResult"
	_preference_result.force_native = true
	_preference_result.dialog_autowrap = true
	_preference_result.confirmed.connect(_close_preference_result)
	_preference_result.canceled.connect(_close_preference_result)
	add_child(_preference_result)
	_renderer_dialog = RendererDialog.new()
	_renderer_dialog.simulation = _simulation
	_renderer_dialog.language = _language
	add_child(_renderer_dialog)
	_renderer_dialog.visibility_changed.connect(_update_launcher_actions)
	_renderer_dialog.status_changed.connect(_update_launcher_actions)
	_renderer_dialog.startup_ready.connect(func(allowed: bool) -> void:
		_update_launcher_actions()
		if allowed and not _launcher_only:
			_start_simulation.call_deferred()
	)
	_refresh_locale()
	_update_launcher_actions()
	_start_stop.grab_focus()


func _toggle_simulation() -> void:
	if _launcher_modal_visible():
		return
	if is_instance_valid(_simulation_window):
		_main.request_stop()
	else:
		_start_simulation()


func _start_simulation() -> void:
	if _launcher_modal_visible():
		return
	if is_instance_valid(_simulation_window):
		_simulation_window.grab_focus()
		return
	var fresh_profile: bool = _new_profile_defaults and str(_simulation.preferences_snapshot()["status"]) == "missing"
	if fresh_profile:
		var defaults: Dictionary = _apply_fresh_profile_co2()
		if not bool(defaults["ok"]):
			_status_key = "R_LAUNCHER_INIT_FAILED"
			_refresh_locale()
			_status.text += "\n" + str(defaults["error"])
			return
	_simulation_window = Window.new()
	_simulation_window.hide()
	_simulation_window.name = "SimulationWindow"
	_simulation_window.force_native = true
	if DisplayServer.get_name() == "Wayland":
		# Keep menus on the simulation surface for touchscreen input
		_simulation_window.gui_embed_subwindows = true
	_simulation_window.title = "Biogenesis-R"
	_simulation_window.size = Vector2i(1180, 820)
	_simulation_window.min_size = Vector2i(800, 660)
	_simulation_window.close_requested.connect(_request_simulation_close)
	_main = Main.new()
	_main.language_changed.connect(_language_changed)
	_main.name = "BiogenesisR"
	_main.startup_seed_text = _seed_text
	_main.startup_resume_managed = _resume_managed
	var settings: Dictionary = _simulation.settings_snapshot()
	_language = str(settings["language"])
	var pending: PackedInt64Array = settings["genesis"]
	if fresh_profile:
		pending = PackedInt64Array([Main.NEW_WORLD_DIMENSIONS.x, Main.NEW_WORLD_DIMENSIONS.y, Main.NEW_WORLD_ATTEMPTS])
	_main.startup_dimensions = Vector2i(pending[0], pending[1])
	_main.startup_attempts = int(pending[2])
	if not _main.initialize_session(_simulation):
		_main.free()
		_main = null
		_simulation_window.free()
		_simulation_window = null
		_status_key = "R_LAUNCHER_INIT_FAILED"
		_refresh_locale()
		return
	_resume_managed = false
	_new_profile_defaults = false
	_main.simulation_closed.connect(_stop_simulation)
	_main.close_cancelled.connect(_cancel_launcher_exit)
	_simulation_window.add_child(_main)
	add_child(_simulation_window)
	_main.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_restore_simulation_geometry()
	if not _simulation_window.visible:
		_simulation_window.popup()
	_update_launcher_actions()
	_status_key = "R_LAUNCHER_RUNNING"
	_refresh_locale()
	var preferences: Dictionary = _simulation.preferences_snapshot()
	if not str(preferences["error"]).is_empty():
		_status.text = _preference_error_status(preferences)


func _apply_fresh_profile_co2() -> Dictionary:
	var draft: BgrSettingsDraft = _simulation.settings_open()
	var state: Dictionary = draft.snapshot()
	if not bool(state["ok"]):
		return state
	var keys: PackedStringArray = state["keys"]
	var index: int = keys.find("INITIAL_CO2")
	if index < 0:
		draft.cancel()
		return {"ok": false, "error": "The native settings draft does not expose INITIAL_CO2"}
	var texts: PackedStringArray = state["texts"]
	texts[index] = str(NEW_PROFILE_CO2)
	var replaced: Dictionary = draft.replace_controls({"texts": texts, "locale": state["locale"], "automatic_backup": state["automatic_backup"], "hardware": state["hardware"], "disable_framebuffer": state["disable_framebuffer"], "automatic_backup_mode": state["automatic_backup_mode"]})
	if not bool(replaced["ok"]):
		return replaced
	return _simulation.settings_apply(draft, _fresh_defaults_locale)


func _fresh_defaults_locale(payload: Dictionary) -> void:
	_language = str(payload["language"])
	_refresh_locale()


func _restore_simulation_geometry() -> void:
	var raw: PackedInt32Array = _simulation.preferences_snapshot()["geometry"]
	var selected: int = get_window().current_screen
	_geometry_notice_key = ""
	var overlap: float = 0.0
	var requested: Rect2 = Rect2(float(raw[0]), float(raw[1]), float(maxi(1, raw[2])), float(maxi(1, raw[3])))
	for screen: int in range(DisplayServer.get_screen_count()):
		var intersection: Rect2 = requested.intersection(Rect2(DisplayServer.screen_get_usable_rect(screen)))
		var area: float = intersection.size.x * intersection.size.y
		if area > overlap:
			overlap = area
			selected = screen
	var usable: Rect2i = DisplayServer.screen_get_usable_rect(selected)
	if usable.size.x < 800 or usable.size.y < 660:
		_geometry_notice_key = "R_LAUNCHER_SCREEN_SMALL"
		_simulation_window.popup_centered()
		return
	var restored: Vector2i = Vector2i(clampi(raw[2], 800, usable.size.x), clampi(raw[3], 660, usable.size.y))
	_simulation_window.size = restored
	_simulation_window.position = Vector2i(clampi(raw[0], usable.position.x, usable.end.x - restored.x), clampi(raw[1], usable.position.y, usable.end.y - restored.y))
	if raw[4] == 6:
		_simulation_window.mode = Window.MODE_MAXIMIZED
	elif raw[4] != 0:
		_geometry_notice_key = "R_LAUNCHER_GEOMETRY_NORMAL"


func _request_simulation_close() -> void:
	if is_instance_valid(_main):
		_main.request_close()


func _stop_simulation() -> void:
	_start_stop.disabled = true
	var closing_main: Main = _main
	if is_instance_valid(closing_main):
		await closing_main.shutdown()
	if not is_inside_tree() or _main != closing_main:
		return
	var session: Dictionary = _simulation.snapshot()
	if session.has("seed"):
		_seed_text = str(session["seed"])
	_main = null
	if is_instance_valid(_simulation_window):
		_simulation_window.hide()
		_simulation_window.queue_free()
	_simulation_window = null
	_update_launcher_actions()
	_status_key = "R_LAUNCHER_STOPPED"
	_refresh_locale()
	if _exit_when_closed:
		get_tree().quit()
	else:
		_start_stop.grab_focus()


func _quit_launcher() -> void:
	if _exit_when_closed:
		return
	if is_instance_valid(_main):
		_exit_when_closed = _main.request_close()
	else:
		get_tree().quit()


func _cancel_launcher_exit() -> void:
	_exit_when_closed = false


func _preference_error_status(preferences: Dictionary) -> String:
	_status_key = "R_LAUNCHER_PREFS_DURABILITY" if bool(preferences.get("durability_uncertain", preferences["replacement_committed"])) else "R_LAUNCHER_PREFS_LOAD_FAILED"
	return MainLabels.message(_status_key, _language)


func _language_changed(language: String) -> void:
	_language = language
	_refresh_locale()


func _refresh_locale() -> void:
	_renderer_dialog.refresh_preferences()
	_renderer_button.text = MainLabels.message("R_RENDERER_BUTTON", _language)
	_renderer_button.tooltip_text = MainLabels.message("R_RENDERER_BUSY", _language)
	_renderer_dialog.apply_locale(_language)
	get_window().title = MainLabels.message("R_LAUNCHER_TITLE", _language)
	_start_stop.text = MainLabels.message("R_LAUNCHER_STOP" if is_instance_valid(_simulation_window) else "R_LAUNCHER_START", _language)
	_start_stop.tooltip_text = MainLabels.message("R_LAUNCHER_ACTION_HELP", _language)
	_quit_button.text = MainLabels.message("R_LAUNCHER_QUIT", _language)
	_import_button.text = MainLabels.message("R_LAUNCHER_IMPORT", _language)
	_import_button.tooltip_text = MainLabels.message("R_LAUNCHER_IMPORT_HELP", _language)
	_reset_button.text = MainLabels.message("R_PREF_RESET", _language)
	_restore_button.text = MainLabels.message("R_PREF_RESTORE", _language)
	_reset_button.tooltip_text = MainLabels.message("R_PREF_ACTION_HELP", _language)
	_restore_button.tooltip_text = MainLabels.message("R_PREF_ACTION_HELP", _language)
	_status.text = MainLabels.message(_status_key, _language)
	_geometry_notice = "" if _geometry_notice_key.is_empty() else MainLabels.message(_geometry_notice_key, _language)
	if _status_key == "R_LAUNCHER_RUNNING" and not _geometry_notice.is_empty():
		_status.text += "\n" + _geometry_notice
	_import_dialog.title = MainLabels.message("R_LAUNCHER_IMPORT_TITLE", _language)
	_import_dialog.filters = PackedStringArray(["*.xml ; " + MainLabels.message("R_LAUNCHER_XML_FILTER", _language), FileFilterLabels.filter_entry("*", "T_ALL_FILES", _language)])
	_import_dialog.ok_button_text = MainLabels.message("R_LAUNCHER_IMPORT_APPROVE", _language)
	_import_dialog.get_cancel_button().text = MainLabels.message("T_CANCEL", _language)
	_import_result.ok_button_text = MainLabels.message("R_LAUNCHER_RESULT_OK", _language)
	if _import_result_state.is_empty():
		return
	if bool(_import_result_state["applied"]) and bool(_import_result_state["ok"]):
		_import_result.title = MainLabels.message("R_LAUNCHER_IMPORTED_TITLE", _language)
		_import_result.dialog_text = str(_import_result_state["notice"])
	elif bool(_import_result_state["applied"]):
		_import_result.title = MainLabels.message("R_LAUNCHER_IMPORT_ATTENTION_TITLE", _language)
		_import_result.dialog_text = MainLabels.message("R_LAUNCHER_IMPORT_DURABILITY", _language) + "\n" + str(_import_result_state["error"])
	else:
		_import_result.title = MainLabels.message("R_LAUNCHER_IMPORT_FAILED_TITLE", _language)
		_import_result.dialog_text = str(_import_result_state["error"]) + "\n" + MainLabels.message("R_LAUNCHER_IMPORT_RETAINED", _language)


func _choose_preferences() -> void:
	if is_instance_valid(_simulation_window) or _launcher_modal_visible():
		return
	_import_dialog.popup_centered_clamped(Vector2i(760, 520))
	_update_launcher_actions()
	_import_dialog.get_line_edit().grab_focus()


func _cancel_import() -> void:
	_import_dialog.hide()
	_update_launcher_actions()
	_import_button.grab_focus()


func _import_preferences(path: String) -> void:
	_import_dialog.hide()
	var result: Dictionary = _simulation.preferences_import(path)
	_import_result_state = result.duplicate()
	_language = str(_simulation.settings_snapshot()["language"])
	var applied: bool = bool(result["applied"])
	if applied and bool(result["ok"]):
		_status_key = "R_LAUNCHER_IMPORTED"
	elif applied:
		_status_key = "R_LAUNCHER_IMPORT_ATTENTION"
	else:
		_status_key = "R_LAUNCHER_IMPORT_FAILED"
	_refresh_locale()
	_import_result.popup_centered_clamped(Vector2i(620, 240))
	_update_launcher_actions()
	_import_result.get_ok_button().grab_focus()


func _close_import_result() -> void:
	_import_result.hide()
	_update_launcher_actions()
	if not _import_button.disabled:
		_import_button.grab_focus()



func _launcher_modal_visible() -> bool:
	return _import_dialog.visible or _import_result.visible or _preference_confirm.visible or _preference_result.visible or (is_instance_valid(_renderer_dialog) and _renderer_dialog.is_busy())


func _update_launcher_actions() -> void:
	var modal: bool = _launcher_modal_visible()
	var running: bool = is_instance_valid(_simulation_window)
	if is_instance_valid(_renderer_dialog):
		_renderer_button.disabled = modal or bool(_simulation.preferences_snapshot()["session_ready"])
		_renderer_notice.text = _renderer_dialog.notice_text()
		_renderer_notice.visible = not _renderer_notice.text.is_empty()
	_start_stop.disabled = modal
	_import_button.disabled = modal or running
	_reset_button.disabled = modal or running
	_restore_button.disabled = modal or running or not _reset_backup_available


func _choose_preference_action(action: StringName) -> void:
	if action not in [&"reset", &"restore"] or is_instance_valid(_simulation_window) or _launcher_modal_visible():
		return
	if action == &"restore" and not _reset_backup_available:
		return
	_preference_action = action
	_preference_action_language = _language
	_preference_focus = _reset_button if action == &"reset" else _restore_button
	var title_key: String = "R_PREF_RESET_TITLE" if action == &"reset" else "R_PREF_RESTORE_TITLE"
	var question_key: String = "R_PREF_RESET_QUESTION" if action == &"reset" else "R_PREF_RESTORE_QUESTION"
	_preference_confirm.title = MainLabels.message(title_key, _preference_action_language)
	_preference_confirm.dialog_text = MainLabels.message(question_key, _preference_action_language)
	_preference_confirm.ok_button_text = MainLabels.message(title_key, _preference_action_language)
	_preference_confirm.get_cancel_button().text = MainLabels.message("T_CANCEL", _preference_action_language)
	_preference_confirm.popup_centered_clamped(Vector2i(620, 260))
	_update_launcher_actions()
	_preference_confirm.get_cancel_button().grab_focus()


func _cancel_preference_action() -> void:
	_preference_action = &""
	_preference_confirm.hide()
	_update_launcher_actions()
	if is_instance_valid(_preference_focus) and not _preference_focus.disabled:
		_preference_focus.grab_focus()


func _apply_preference_action() -> void:
	if _preference_action not in [&"reset", &"restore"]:
		return
	var action: StringName = _preference_action
	_preference_action = &""
	_preference_confirm.hide()
	var result: Dictionary = _simulation.preferences_reset() if action == &"reset" else _simulation.preferences_restore()
	_reset_backup_available = bool(result["reset_backup_available"])
	_language = str(_simulation.settings_snapshot()["language"])
	if bool(result["applied"]) and bool(result["ok"]):
		_status_key = "R_LAUNCHER_STOPPED"
	elif bool(result["applied"]):
		_preference_error_status(result)
	_refresh_locale()
	_preference_result.title = MainLabels.message("R_PREF_RESULT_TITLE", _preference_action_language)
	_preference_result.ok_button_text = MainLabels.message("R_LAUNCHER_RESULT_OK", _preference_action_language)
	if bool(result["applied"]) and bool(result["ok"]):
		var done_key: String = "R_PREF_RESET_DONE" if action == &"reset" else "R_PREF_RESTORE_DONE"
		var recovery_key: String = "R_PREF_RECOVERY_AVAILABLE" if _reset_backup_available else "R_PREF_RECOVERY_NONE"
		_preference_result.dialog_text = MainLabels.message(done_key, _preference_action_language) + "\n" + MainLabels.message(recovery_key, _preference_action_language)
	else:
		var result_key: String = "R_PREF_ACTION_ATTENTION" if bool(result["applied"]) else "R_PREF_ACTION_FAILED"
		_preference_result.dialog_text = MainLabels.message(result_key, _preference_action_language) + "\n" + str(result["error"])
	_preference_result.popup_centered_clamped(Vector2i(620, 260))
	_update_launcher_actions()
	_preference_result.get_ok_button().grab_focus()


func _close_preference_result() -> void:
	_preference_result.hide()
	_update_launcher_actions()
	if is_instance_valid(_preference_focus) and not _preference_focus.disabled:
		_preference_focus.grab_focus()
	else:
		_reset_button.grab_focus()


func _exit_tree() -> void:
	if is_instance_valid(_main):
		_main.shutdown(true)
