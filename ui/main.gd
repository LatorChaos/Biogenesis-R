extends Control

signal simulation_closed
signal close_cancelled
signal language_changed(language: String)
signal world_save_requested(reason: WorldSaveReason)

enum WorldSaveReason { CREATE, RESET, QUIT }

const NEW_WORLD_DIMENSIONS: Vector2i = Vector2i(3500, 3500)
const NEW_WORLD_ATTEMPTS: int = 3125

const WorldCanvas = preload("res://ui/world_canvas.gd")
const OrganismControls = preload("res://ui/organism_controls.gd")
const GenomePreview = preload("res://ui/genome_preview.gd")
const LaboratoryWindow = preload("res://ui/laboratory_window.gd")
const SettingsWindow = preload("res://ui/settings_window.gd")
const NetworkWindow = preload("res://ui/network_window.gd")
const NetworkLabels = preload("res://ui/network_labels.gd")
const StatisticsWindow = preload("res://ui/statistics_window.gd")
const StatisticsLabels = preload("res://ui/statistics_labels.gd")
const MainLabels = preload("res://ui/main_labels.gd")
const RESUME_ICON: Texture2D = preload("res://ui/assets/resume.svg")
const PAUSE_ICON: Texture2D = preload("res://ui/assets/pause.svg")
const WORLD_ACTION_ICONS: Texture2D = preload("res://ui/assets/world_actions.svg")
const MAIN_ACTION_LABELS: Dictionary = {
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
	17: ["T_NEW", "T_NEW_WORLD"],
	18: ["T_QUIT", "T_QUIT_PROGRAM"],
	19: ["T_STATISTICS", "T_WORLD_STATISTICS"],
	20: ["T_GENETIC_LABORATORY", "T_GENETIC_LABORATORY"],
	21: ["T_PARAMETERS", "T_EDIT_PARAMETERS"]
}
const QUICK_ACTION_IDS: Array[int] = [12, 14, 6, 7, 10]
const FileFilterLabels = preload("res://ui/file_filter_labels.gd")
const HelpAbout = preload("res://ui/help_about.gd")
const VersionCheck = preload("res://ui/version_dialog.gd")
const LinePathPresentation = preload("res://ui/line_path_presentation.gd")
const EnduranceMetrics = preload("res://ui/endurance_metrics.gd")
var _endurance: EnduranceMetrics
const NativeFrameLoop = preload("res://ui/native_frame_loop.gd")
var _native_configuration_cache: Dictionary = {}
const AdaptivePacing = preload("res://ui/adaptive_pacing.gd")
const AmbientShell = preload("res://ui/ambient_shell.gd")
const DisplayPreferences = preload("res://ui/display_preferences.gd")
const CinemaDirector = preload("res://ui/cinema_director.gd")
const LivingCards = preload("res://ui/living_cards.gd")
const FULLSCREEN_LABELS: Dictionary = {
	"en": ["Fullscreen [F11]", "Windowed [F11]"],
	"es": ["Pantalla completa [F11]", "Ventana [F11]"],
	"ca": ["Pantalla completa [F11]", "Finestra [F11]"],
	"de": ["Vollbild [F11]", "Fenster [F11]"],
	"pt": ["Ecrã inteiro [F11]", "Janela [F11]"],
	"ru": ["Полный экран [F11]", "Оконный режим [F11]"]
}
var _ambient_shell: AmbientShell
var _fullscreen_button: Button
var _display_preferences: DisplayPreferences = DisplayPreferences.new()
var _windowed_geometry: Rect2i
var _windowed_mode: Window.Mode = Window.MODE_WINDOWED
var _cinema_enabled: bool = true
var _continuous_display: bool = true
var _continuous_display_checkbox: CheckBox
var _continuous_display_help: Label
var _manual_camera_until: float = 0.0
var _director: CinemaDirector = CinemaDirector.new()
var _living_cards: LivingCards
var _scene_poll_elapsed: float = 1.0
var _cinema_kind: String = ""
var _presentation_clock_usec: int = 0
var _pacing: AdaptivePacing = AdaptivePacing.new()
var _pacing_sample_usec: int = 0
var _overload_paused: bool = false
const MAX_TICKS_PER_FRAME: int = 16
const SIMULATION_BUDGET_USEC: int = 8000

var _network_window: NetworkWindow
var _network_managers: Dictionary[int, WeakRef] = {}
var _network_menu: MenuButton
var _network_button: Button
var _network_status_label: Label
var _network_packet: Dictionary = {}
var _network_error: String = ""
var _settings_hidden_windows: Array[Window] = []
var _settings_embedding_owner: WeakRef
var _settings_window: SettingsWindow
var _settings_report: RichTextLabel
var _settings_button: Button
var _statistics_button: Button
var _statistics_window: StatisticsWindow
var _help_menu: MenuButton
var _help_language: String = "en"
var _about_window: HelpAbout
var _version_window: VersionCheck
var _crisp_lines: CheckButton
var _stored_delay_ms: int = 2
var _preference_failure: ConfirmationDialog
var _preference_close_kind: String = ""
var _preference_failure_detail: String = ""
var _discard_preferences_button: Button
var _simulation: BgrSimulation
var _state: Dictionary = {}
var _organism_controls: OrganismControls
var _genetics_menu: MenuButton
var _world_context: PopupMenu
var _placement_hint: Label
var _placement_mode: String = ""
var _placement_position: Vector2 = Vector2.ZERO
var _file_dialog: FileDialog
var _recovery_dialog: ConfirmationDialog
var _recovery_choice: String = ""
var _recovery_detail: String = ""
var _open_manual_button: Button
var _overwrite_dialog: ConfirmationDialog
var _file_operation: String = ""
var _file_position: Vector2 = Vector2.ZERO
var _restore_running: bool = false
var _save_path: String = ""
var _world_path: String = ""
var _pending_image: BgrImageExport
var _save_filename: String = ""
var _world_menu: MenuButton
var _actions_menu: MenuButton
var _world_tools_menu: MenuButton
var _quick_buttons: Array[Button] = []
var _backup_key: int = KEY_B
var _save_key: int = KEY_S
var _help_label: Label
var _canvas: WorldCanvas
var _pause_button: Button
var _main_language: String = "en"
var _pause_action_key: int = KEY_R
var _resume_key: int = KEY_R
var _pause_key: int = KEY_P
var _step_button: Button
var _reset_button: Button
var _seed_input: LineEdit
var _setup_labels: Array[Label] = []
var _disclosure_label: Label
var _view_zoom: float = 1.0
var _width_input: LineEdit
var _height_input: LineEdit
var _attempts_input: LineEdit
var _mode_label: Label
var _metrics_label: Label
var _timing_label: Label
var _zoom_label: Label
var _error_label: Label
var _error_message: String = ""
var _error_message_key: String = ""
var _network_error_packet: Dictionary = {}
var _retained_input_count: int = 0
var _automatic_label: Label
var _automatic_status: Dictionary = {}
var _automatic_source_active: bool = false
var _pending_ticks: float = 0.0
var _last_clock_usec: int = 0
var _last_step_ms: float = 0.0
var _runtime_failed: bool = false
var startup_seed_text: String = ""
var startup_resume_managed: bool = false
var _startup_recovery_succeeded: bool = false
var startup_dimensions: Vector2i = NEW_WORLD_DIMENSIONS
var startup_attempts: int = NEW_WORLD_ATTEMPTS
var _decision_dialog: ConfirmationDialog
var _save_world_button: Button
var _decision_operation: String = ""
var _decision_values: PackedInt64Array = PackedInt64Array()
var _decision_restore_running: bool = false
var _awaiting_world_save: bool = false
var _creating_world: bool = false
var _creation_generation: int = 0
var _shutting_down: bool = false


func _ready() -> void:
	if get_window() == get_tree().root:
		get_tree().auto_accept_quit = false
		get_window().close_requested.connect(request_close)
	get_window().title = "Biogenesis-R"
	get_window().min_size = Vector2i(800, 660)
	theme = _make_theme()
	_build_interface()
	world_save_requested.connect(_save_world_for_decision)
	if not startup_seed_text.is_empty():
		_seed_input.text = startup_seed_text
	_width_input.text = str(startup_dimensions.x)
	_height_input.text = str(startup_dimensions.y)
	_attempts_input.text = str(startup_attempts)
	if _simulation == null:
		_simulation = BgrSimulation.new()
	var effective_settings: Dictionary = _simulation.settings_snapshot()
	_main_language = str(effective_settings["language"])
	_update_world_shortcuts(_main_language)
	_refresh_status_locale()
	_statistics_button.text = StatisticsLabels.message("T_STATISTICS", str(effective_settings["language"]))
	_apply_help_locale(str(effective_settings["language"]))
	_stored_delay_ms = int(effective_settings["values"]["DELAY"])
	_reset_pacing()
	var crisp: bool = bool(_simulation.settings_snapshot()["crisp_lines"])
	_crisp_lines.set_pressed_no_signal(crisp)
	_canvas.set_crisp_lines(crisp)
	_last_clock_usec = _clock_usec()
	_update_automatic_status(_simulation.configure_automatic_backups(OS.get_user_data_dir(), _world_path))
	_initialize_world()
	if startup_resume_managed and not _state.is_empty():
		var recovered: Dictionary = _simulation.recover_startup_world()
		if _accept_result(recovered):
			_world_path = ""
			_loaded_world_ready()
			_startup_recovery_succeeded = true
		else:
			_update_automatic_status(_simulation.automatic_backup_status())
			if bool(_automatic_status.get("manual_checkpoint_pending", false)):
				_show_recovery_choice.call_deferred("older")
	_pump_network()
	var preferences: Dictionary = _simulation.preferences_snapshot()
	if not str(preferences["error"]).is_empty():
		_show_error(str(preferences["error"]))
	var display_result: Dictionary = _display_preferences.load_settings()
	_cinema_enabled = bool(_display_preferences.values["cinema_enabled"])
	_continuous_display = bool(_display_preferences.values["continuous_display"])
	_continuous_display_checkbox.set_pressed_no_signal(_continuous_display)
	_ambient_shell.idle_seconds = float(_display_preferences.values["idle_seconds"])
	_ambient_shell.set_cinema_enabled(_cinema_enabled)
	_director.hold_seconds = float(_display_preferences.values["camera_hold_seconds"])
	_apply_cinema_preferences()
	_living_cards.slide_interval_seconds = float(_display_preferences.values["slide_interval_seconds"])
	_living_cards.slide_dwell_seconds = float(_display_preferences.values["slide_dwell_seconds"])
	if not bool(display_result["ok"]):
		_show_error(str(display_result["error"]))
	_start_endurance()
	_apply_display_mode.call_deferred()
	var presentation: Control = LinePathPresentation.new()
	presentation.name = "LinePathPresentation"
	add_child(presentation)
	_presentation_clock_usec = _clock_usec()


func initialize_session(session: BgrSimulation) -> bool:
	if is_inside_tree() or session == null:
		return false
	_simulation = session
	return true


func applied_configuration() -> PackedInt64Array:
	if _state.is_empty():
		return PackedInt64Array()
	return PackedInt64Array([int(_state["seed"]), int(_state["width"]), int(_state["height"]), int(_state["initial_attempts"])])


func _make_theme() -> Theme:
	var result: Theme = Theme.new()
	result.default_font_size = 16
	result.set_color("font_color", "Label", Color("dce8ee"))
	result.set_color("font_color", "Button", Color("dce8ee"))
	result.set_color("font_color", "LineEdit", Color("e7f2f6"))
	result.set_color("caret_color", "LineEdit", Color("93ddca"))
	for state: String in ["normal", "hover", "pressed", "disabled", "focus"]:
		var box: StyleBoxFlat = StyleBoxFlat.new()
		box.bg_color = Color("14272d")
		box.border_color = Color("29484e")
		if state == "hover" or state == "focus":
			box.bg_color = Color("243c4d")
			box.border_color = Color("83cdb6")
		elif state == "pressed":
			box.bg_color = Color("355b64")
		elif state == "disabled":
			box.bg_color = Color("111e29")
		box.set_border_width_all(1)
		box.set_corner_radius_all(10)
		box.content_margin_left = 12.0
		box.content_margin_right = 12.0
		box.content_margin_top = 8.0
		box.content_margin_bottom = 8.0
		result.set_stylebox(state, "Button", box)
		result.set_stylebox(state, "LineEdit", box)
	return result


func _build_interface() -> void:
	var margin: MarginContainer = MarginContainer.new()
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for side: String in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 16)
	add_child(margin)
	var column: VBoxContainer = VBoxContainer.new()
	column.add_theme_constant_override("separation", 6)
	margin.add_child(column)
	var header: HBoxContainer = HBoxContainer.new()
	column.add_child(header)
	var title: Label = _label("Biogenesis-R", 26)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title)
	_mode_label = _label(MainLabels.message("R_INITIALIZING", _main_language), 15)
	_mode_label.add_theme_color_override("font_color", Color("93ddca"))
	header.add_child(_mode_label)
	_disclosure_label = _label(MainLabels.message("R_DISCLOSURE", _main_language), 13)
	_disclosure_label.name = "ConversionDisclosure"
	_disclosure_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_disclosure_label.add_theme_color_override("font_color", Color("9eb4c3"))
	column.add_child(_disclosure_label)
	var settings: HFlowContainer = HFlowContainer.new()
	settings.add_theme_constant_override("h_separation", 12)
	settings.add_theme_constant_override("v_separation", 8)
	column.add_child(settings)
	_seed_input = _field(settings, "R_SETUP_SEED", "827364", "SeedInput", 160)
	_width_input = _field(settings, "R_SETUP_WIDTH", str(NEW_WORLD_DIMENSIONS.x), "WidthInput", 120)
	_height_input = _field(settings, "R_SETUP_HEIGHT", str(NEW_WORLD_DIMENSIONS.y), "HeightInput", 120)
	_attempts_input = _field(settings, "R_SETUP_ATTEMPTS", str(NEW_WORLD_ATTEMPTS), "AttemptsInput", 140)
	var apply: Button = _button("Create world", "CreateWorldButton", _request_create_world)
	apply.tooltip_text = MainLabels.message("T_NEW_WORLD", _main_language)
	settings.add_child(apply)
	var toolbar: HFlowContainer = HFlowContainer.new()
	toolbar.add_theme_constant_override("h_separation", 8)
	toolbar.add_theme_constant_override("v_separation", 6)
	column.add_child(toolbar)
	_continuous_display_checkbox = CheckBox.new()
	_continuous_display_checkbox.name = "ContinuousDisplay"
	_continuous_display_checkbox.custom_minimum_size.y = 48.0
	_continuous_display_checkbox.set_pressed_no_signal(_continuous_display)
	_continuous_display_checkbox.toggled.connect(_set_continuous_display)
	column.add_child(_continuous_display_checkbox)
	_continuous_display_help = _label("", 12)
	_continuous_display_help.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	column.add_child(_continuous_display_help)
	_pause_button = _button("Resume [P / Space]", "PauseButton", func() -> void: _toggle_pause("toolbar"))
	toolbar.add_child(_pause_button)
	for index: int in range(QUICK_ACTION_IDS.size()):
		if index == 4:
			_network_button = _button("", "ManageConnectionsButton", _open_network.bind(1))
			toolbar.add_child(_network_button)
		var quick: Button = _button("", ["SaveWorldButton", "BackupWorldButton", "IncreaseCO2Button", "DecreaseCO2Button", "AbortTrackingButton"][index], _on_world_action.bind(QUICK_ACTION_IDS[index]))
		var icon: AtlasTexture = AtlasTexture.new()
		icon.atlas = WORLD_ACTION_ICONS
		icon.region = Rect2(index * 16, 0, 16, 16)
		quick.icon = icon
		_quick_buttons.append(quick)
		toolbar.add_child(quick)
	_step_button = _button("Step [N]", "StepButton", _single_step)
	toolbar.add_child(_step_button)
	_reset_button = _button("Reset [R]", "ResetButton", _reset_world)
	_reset_button.tooltip_text = "Replay the applied seed with current dimensions and initial attempts. This replaces retained tree and corridor owners."
	toolbar.add_child(_reset_button)
	_world_menu = MenuButton.new()
	_world_menu.name = "WorldActionsMenu"
	var world_popup: PopupMenu = _world_menu.get_popup()
	for command: int in [17, 16, 11, 12, 13, 14]:
		world_popup.add_item("", command)
	world_popup.add_separator()
	world_popup.add_item("", 15)
	world_popup.add_item("", 18)
	world_popup.id_pressed.connect(_on_world_action)
	world_popup.window_input.connect(_on_world_popup_input)
	_world_menu.about_to_popup.connect(_refresh_main_actions)
	toolbar.add_child(_world_menu)
	_actions_menu = MenuButton.new()
	_actions_menu.name = "GlobalActionsMenu"
	var actions_popup: PopupMenu = _actions_menu.get_popup()
	for command: int in [6, 7, 8, 9, 4, 5]:
		actions_popup.add_item("", command)
	actions_popup.id_pressed.connect(_on_world_action)
	actions_popup.window_input.connect(_on_actions_popup_input)
	_actions_menu.about_to_popup.connect(_refresh_main_actions)
	toolbar.add_child(_actions_menu)
	_world_tools_menu = MenuButton.new()
	_world_tools_menu.name = "WorldToolsMenu"
	var tools_popup: PopupMenu = _world_tools_menu.get_popup()
	for command: int in [19, 20, 21]:
		tools_popup.add_item("", command)
	tools_popup.add_separator()
	tools_popup.add_item("", 10)
	tools_popup.id_pressed.connect(_on_world_action)
	tools_popup.window_input.connect(_on_world_tools_popup_input)
	_world_tools_menu.about_to_popup.connect(_refresh_main_actions)
	toolbar.add_child(_world_tools_menu)
	_network_menu = MenuButton.new()
	_network_menu.name = "NetworkMenu"
	var network_popup: PopupMenu = _network_menu.get_popup()
	network_popup.add_item("", 0)
	network_popup.add_item("", 1)
	network_popup.id_pressed.connect(_open_network)
	network_popup.window_input.connect(_on_network_popup_input)
	_network_menu.about_to_popup.connect(_refresh_main_actions)
	toolbar.add_child(_network_menu)
	_genetics_menu = MenuButton.new()
	_genetics_menu.name = "GeneticsMenu"
	_genetics_menu.text = "Genetics"
	var genetics_popup: PopupMenu = _genetics_menu.get_popup()
	for entry: Array in [["Laboratory from clipboard", 0], ["Empty laboratory", 1], ["Copy selected genotype", 2], ["Examine selected genes", 3], ["Paste genotype", 4], ["Create random organism", 5], ["Import genotype", 6], ["Export selected genotype", 7], ["Save selected image", 8]]:
		genetics_popup.add_item(str(entry[0]), int(entry[1]))
	genetics_popup.id_pressed.connect(_on_genetics_action)
	toolbar.add_child(_genetics_menu)
	_settings_button = _button("Settings", "SettingsButton", _open_settings)
	toolbar.add_child(_settings_button)
	_statistics_button = _button("Statistics", "StatisticsButton", _open_statistics)
	toolbar.add_child(_statistics_button)
	_help_menu = MenuButton.new()
	_help_menu.name = "HelpMenu"
	_help_menu.focus_mode = Control.FOCUS_ALL
	var help_popup: PopupMenu = _help_menu.get_popup()
	help_popup.add_item(HelpAbout.message("T_USER_MANUAL", "en"), 0)
	help_popup.add_item(HelpAbout.message("T_CHECK_LAST_VERSION", "en"), 2)
	help_popup.add_item(HelpAbout.message("T_ABOUT", "en"), 1)
	help_popup.id_pressed.connect(_on_help_action)
	help_popup.window_input.connect(_on_help_popup_input)
	_help_menu.about_to_popup.connect(_refresh_help_locale)
	toolbar.add_child(_help_menu)
	for menu: MenuButton in [_world_menu, _actions_menu, _world_tools_menu, _network_menu, _genetics_menu, _help_menu]:
		menu.flat = false
		menu.custom_minimum_size.y = 48.0
	toolbar.add_child(_button("Fit [F]", "FitButton", _fit_world))
	toolbar.add_child(_button("Zoom -", "ZoomOutButton", _zoom_out))
	toolbar.add_child(_button("Zoom +", "ZoomInButton", _zoom_in))
	_network_status_label = Label.new()
	_network_status_label.name = "NetworkStatusLabel"
	_network_status_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_network_status_label.hide()
	column.add_child(_network_status_label)
	_crisp_lines = CheckButton.new()
	_crisp_lines.name = "CrispLines"
	_crisp_lines.text = "Crisp lines"
	_crisp_lines.tooltip_text = "Use sharp one-pixel lines for larger worlds."
	_crisp_lines.button_pressed = false
	toolbar.add_child(_crisp_lines)
	toolbar.add_child(_button("Quit [Esc]", "QuitButton", _quit))
	_error_label = _label("", 14)
	_error_label.name = "ErrorLabel"
	_error_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_error_label.add_theme_color_override("font_color", Color("ffb2a9"))
	_error_label.visible = false
	column.add_child(_error_label)
	_automatic_label = _label("", 13)
	_automatic_label.name = "AutomaticBackupStatus"
	_automatic_label.clip_text = true
	_automatic_label.mouse_filter = Control.MOUSE_FILTER_PASS
	_automatic_label.hide()
	column.add_child(_automatic_label)
	_settings_report = RichTextLabel.new()
	_settings_report.name = "SettingsRetainedFields"
	_settings_report.custom_minimum_size.y = 90
	_settings_report.selection_enabled = true
	_settings_report.scroll_active = true
	_settings_report.fit_content = false
	_settings_report.hide()
	column.add_child(_settings_report)
	_preference_failure = ConfirmationDialog.new()
	_preference_failure.hide()
	_preference_failure.name = "PreferenceSaveFailure"
	_preference_failure.title = MainLabels.message("R_PREFERENCES_SAVE_FAILED", _main_language)
	_preference_failure.dialog_hide_on_ok = false
	_preference_failure.dialog_autowrap = true
	_preference_failure.ok_button_text = MainLabels.message("R_RETRY_SAVE", _main_language)
	_preference_failure.get_ok_button().name = "RetryPreferenceSave"
	_preference_failure.get_cancel_button().name = "CancelPreferenceClose"
	_discard_preferences_button = _preference_failure.add_button(MainLabels.message("R_CONTINUE_WITHOUT_SAVING", _main_language), true, "discard")
	_discard_preferences_button.name = "DiscardPreferenceSave"
	_preference_failure.confirmed.connect(_retry_preference_close)
	_preference_failure.canceled.connect(_cancel_preference_close)
	_preference_failure.custom_action.connect(_preference_close_action)
	add_child(_preference_failure)
	_canvas = WorldCanvas.new()
	_canvas.name = "WorldCanvas"
	_canvas.underwater_background = true
	_canvas.focus_mode = Control.FOCUS_ALL
	_canvas.size_flags_vertical = Control.SIZE_EXPAND_FILL
	# Let wrapped controls and footer hints reclaim height before the canvas overflows
	_canvas.custom_minimum_size = Vector2(0.0, 32.0)
	_canvas.view_changed.connect(_on_view_changed)
	_canvas.organism_picked.connect(_on_organism_picked)
	column.add_child(_canvas)
	_placement_hint = _label("", 13)
	_placement_hint.name = "PlacementHint"
	_placement_hint.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_placement_hint.clip_text = true
	_placement_hint.visible = false
	_placement_hint.add_theme_color_override("font_outline_color", Color.BLACK)
	_placement_hint.add_theme_constant_override("outline_size", 4)
	_canvas.add_child(_placement_hint)
	_placement_hint.set_anchors_and_offsets_preset(Control.PRESET_TOP_WIDE)
	_placement_hint.offset_left = 12.0
	_placement_hint.offset_top = 12.0
	_placement_hint.offset_right = -12.0
	_placement_hint.offset_bottom = 36.0
	_crisp_lines.toggled.connect(_crisp_changed)
	_organism_controls = OrganismControls.new()
	_organism_controls.action_requested.connect(_on_organism_action)
	_organism_controls.genetics_requested.connect(_on_selected_genetics_action)
	_organism_controls.preview_copy_requested.connect(func() -> void: _accept_result(_simulation.copy_preview()))
	_organism_controls.tracking_requested.connect(_on_tracking_requested)
	_organism_controls.abort_tracking_requested.connect(_on_abort_tracking_requested)
	column.add_child(_organism_controls)
	var footer: HBoxContainer = HBoxContainer.new()
	column.add_child(footer)
	_metrics_label = _label(MainLabels.message("R_WAITING", _main_language), 14)
	_metrics_label.name = "MetricsLabel"
	_metrics_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_metrics_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	footer.add_child(_metrics_label)
	_zoom_label = _label("", 13)
	_zoom_label.name = "ZoomLabel"
	footer.add_child(_zoom_label)
	_timing_label = _label("", 13)
	_timing_label.name = "TimingLabel"
	_timing_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	column.add_child(_timing_label)
	_help_label = _label("", 12)
	_help_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_help_label.add_theme_color_override("font_color", Color("9eb4c3"))
	column.add_child(_help_label)
	_world_context = PopupMenu.new()
	_world_context.name = "WorldContext"
	_world_context.add_item("Paste genotype", 4)
	_world_context.add_item("Create random organism", 5)
	_world_context.add_item("Import genotype", 6)
	_world_context.add_separator()
	_world_context.add_item("Laboratory", 0)
	_world_context.id_pressed.connect(_on_world_context_action)
	add_child(_world_context)
	_file_dialog = FileDialog.new()
	FileFilterLabels.fit_filter_labels(_file_dialog)
	_file_dialog.name = "GenotypeFileDialog"
	_file_dialog.access = FileDialog.ACCESS_FILESYSTEM
	var documents: String = OS.get_system_dir(OS.SYSTEM_DIR_DOCUMENTS)
	_file_dialog.current_dir = documents if DirAccess.dir_exists_absolute(documents) else OS.get_user_data_dir()
	_file_dialog.overwrite_warning_enabled = false
	_file_dialog.file_selected.connect(_on_genotype_file_selected)
	_file_dialog.window_input.connect(_capture_save_filename)
	_file_dialog.canceled.connect(_finish_file_operation)
	add_child(_file_dialog)
	_overwrite_dialog = ConfirmationDialog.new()
	_overwrite_dialog.name = "GenotypeOverwriteDialog"
	_overwrite_dialog.title = MainLabels.message("T_FILE_EXISTS", _main_language)
	_overwrite_dialog.dialog_autowrap = true
	_overwrite_dialog.get_ok_button().name = "ConfirmFileOverwrite"
	_overwrite_dialog.get_cancel_button().name = "CancelFileOverwrite"
	_overwrite_dialog.confirmed.connect(_save_export)
	_overwrite_dialog.canceled.connect(_finish_file_operation)
	add_child(_overwrite_dialog)
	_recovery_dialog = ConfirmationDialog.new()
	_recovery_dialog.name = "ManualRecoveryChoice"
	_recovery_dialog.dialog_autowrap = true
	_recovery_dialog.get_ok_button().name = "ConfirmManualRecoveryChoice"
	_recovery_dialog.get_cancel_button().name = "CancelManualRecoveryChoice"
	_open_manual_button = _recovery_dialog.add_button("", true, "open")
	_open_manual_button.name = "OpenManualRecoveryChoice"
	_recovery_dialog.confirmed.connect(_confirm_recovery_choice)
	_recovery_dialog.canceled.connect(_cancel_recovery_choice)
	_recovery_dialog.custom_action.connect(_recovery_custom_action)
	add_child(_recovery_dialog)
	_decision_dialog = ConfirmationDialog.new()
	_decision_dialog.name = "WorldDecisionDialog"
	_decision_dialog.dialog_autowrap = true
	_decision_dialog.get_ok_button().name = "ConfirmWorldDecision"
	_decision_dialog.get_cancel_button().name = "CancelWorldDecision"
	_save_world_button = _decision_dialog.add_button(MainLabels.message("T_SAVE_WORLD", _main_language), true, "save")
	_save_world_button.name = "SaveWorldDecision"
	_decision_dialog.confirmed.connect(_confirm_world_decision)
	_decision_dialog.canceled.connect(_cancel_world_decision)
	_decision_dialog.custom_action.connect(_world_decision_action)
	add_child(_decision_dialog)
	margin.name = "WorldTools"
	_disclosure_label.hide()
	_help_label.hide()
	_ambient_shell = AmbientShell.new()
	add_child(_ambient_shell)
	_ambient_shell.configure(_canvas, margin, _organism_controls)
	_ambient_shell.pause_requested.connect(func() -> void: _toggle_pause("ambient"))
	_ambient_shell.fit_requested.connect(_fit_world)
	_ambient_shell.cinema_toggled.connect(_set_cinema_enabled)
	_canvas.manual_view_changed.connect(_manual_camera_override)
	_living_cards = LivingCards.new()
	_living_cards.name = "LivingCards"
	add_child(_living_cards)
	_fullscreen_button = _button("Fullscreen [F11]", "FullscreenButton", _toggle_fullscreen)
	toolbar.add_child(_fullscreen_button)


func _label(text: String, font_size: int) -> Label:
	var label: Label = Label.new()
	label.text = text
	label.add_theme_font_size_override("font_size", font_size)
	return label


func _field(parent: Container, label_key: String, value: String, node_name: String, width: int) -> LineEdit:
	var group: VBoxContainer = VBoxContainer.new()
	var label: String = MainLabels.message(label_key, _main_language)
	var caption: Label = _label(label, 12)
	caption.name = node_name + "Label"
	_setup_labels.append(caption)
	group.add_child(caption)
	var field: LineEdit = LineEdit.new()
	field.name = node_name
	field.accessibility_name = label
	field.text = value
	field.custom_minimum_size.x = width
	field.custom_minimum_size.y = 48.0
	field.select_all_on_focus = true
	group.add_child(field)
	parent.add_child(group)
	return field


func _button(text: String, node_name: String, action: Callable) -> Button:
	var button: Button = Button.new()
	button.name = node_name
	button.text = text
	button.custom_minimum_size = Vector2(48.0, 48.0)
	button.pressed.connect(action)
	return button


func _initialize_world() -> void:
	_accrue_clock()
	var values: PackedInt64Array = _read_world_values()
	if values.size() == 4:
		var result: Dictionary = _simulation.start_session(values[0], values[1], values[2], values[3])
		if result.has("seed"):
			_seed_input.text = str(result["seed"])
		if _accept_result(result):
			_world_ready()


func _read_world_values() -> PackedInt64Array:
	var fields: Array[LineEdit] = [_seed_input, _width_input, _height_input, _attempts_input]
	var error_keys: PackedStringArray = ["R_INTEGER_SEED", "R_INTEGER_WIDTH", "R_INTEGER_HEIGHT", "R_INTEGER_ATTEMPTS"]
	var values: PackedInt64Array = PackedInt64Array()
	for field: LineEdit in fields:
		var text: String = field.text.strip_edges()
		if not text.is_valid_int():
			_show_error("", error_keys[fields.find(field)])
			field.grab_focus()
			return PackedInt64Array()
		var normalized: String = text.trim_prefix("+")
		var negative: bool = normalized.begins_with("-")
		normalized = normalized.trim_prefix("-")
		while normalized.length() > 1 and normalized.begins_with("0"):
			normalized = normalized.substr(1)
		var limit: String = "9223372036854775808" if negative else "9223372036854775807"
		if normalized.length() > limit.length() or (normalized.length() == limit.length() and normalized > limit):
			_show_error("", "R_INTEGER_RANGE")
			field.grab_focus()
			return PackedInt64Array()
		if negative and normalized != "0":
			normalized = "-" + normalized
		values.append(normalized.to_int())
	return values


func _configure_world(values: PackedInt64Array) -> bool:
	var result: Dictionary
	if _state.is_empty():
		var stopped: Dictionary = _simulation.snapshot()
		if not stopped.has("seed") or values[0] == int(stopped["seed"]):
			result = _simulation.start_session(values[0], values[1], values[2], values[3])
		else:
			result = _simulation.configure(values[0], values[1], values[2], values[3])
	elif values[0] == int(_state["seed"]):
		result = _simulation.new_game(values[1], values[2], values[3])
	else:
		result = _simulation.configure(values[0], values[1], values[2], values[3])
	var succeeded: bool = _accept_result(result)
	if succeeded or bool(result.get("new_game_started", false)):
		if _continuous_display:
			_pacing.clear_continuous_sample()
			_pacing_sample_usec = 0
		_pending_ticks = 0.0
		_last_clock_usec = _clock_usec()
	if succeeded:
		_world_ready()
		return true
	return false


func _world_ready() -> void:
	_manual_camera_until = 0.0
	_director.clear()
	_scene_poll_elapsed = 1.0
	if is_instance_valid(_living_cards):
		_living_cards.dismiss()
	_pending_ticks = 0.0
	_reset_pacing()
	_last_clock_usec = _clock_usec()
	_runtime_failed = false
	_error_label.visible = false
	_canvas.fit_world()
	_update_labels()
	get_viewport().gui_release_focus()


func _request_create_world() -> void:
	var values: PackedInt64Array = _read_world_values()
	if values.size() == 4:
		_request_world_decision("create", values)


func _accept_result(result: Dictionary) -> bool:
	var accepted: bool = bool(result.get("ok", false))
	if not accepted and not result.has("segments"):
		_show_error(str(result.get("error", "The simulation returned an unspecified error.")))
		return false
	var segments: PackedFloat64Array = result["segments"]
	var colors: PackedInt32Array = result["colors"]
	var alphas: PackedByteArray = result["alphas"]
	var identities: PackedInt64Array = result["organism_ids"]
	if segments.size() != colors.size() * 4 or colors.size() != identities.size() or colors.size() != alphas.size():
		_show_error("The simulation returned an invalid segment batch.")
		return false
	if not _valid_selection(result):
		_show_error("The simulation returned invalid selected-organism data.")
		return false
	if not _organism_controls.set_genome_preview(result["genotype_preview"], result["genotype_preview_error"]):
		_show_error("The simulation returned an invalid selected-organism preview.")
		return false
	_accrue_clock()
	var was_paused: bool = bool(_state.get("paused", true))
	_state = result
	if _cinema_allowed() and result.get("feeding") is Dictionary:
		var view: Dictionary = _canvas.view_state()
		var span: Vector2 = _canvas.size / float(view["zoom"])
		_director.observe_feeding(result["feeding"], Rect2((view["center"] as Vector2) - span * 0.5, span))
	if bool(_state["paused"]):
		_pacing.clear_continuous_sample()
		_pacing_sample_usec = 0
	elif was_paused:
		if _overload_paused:
			_reset_pacing()
		_pacing_sample_usec = _clock_usec()
	_update_automatic_status(result["automatic_backup"])
	if is_instance_valid(_statistics_window) and not bool(result.get("statistics_open", false)):
		_close_statistics()
	if bool(result.get("runtime_error", false)):
		_runtime_failed = true
	_canvas.set_snapshot(segments, colors, float(result["width"]), float(result["height"]), alphas)
	_canvas.set_selection(result["selected"])
	_organism_controls.set_selection(result["selected"], bool(result["selected_is_tracked"]), bool(result["tracking"]))
	var tracking_path: PackedInt32Array = result["tracking_path"]
	for index: int in range(0, tracking_path.size(), 2):
		_canvas.follow_organism(PackedFloat64Array([tracking_path[index], tracking_path[index + 1]]), float(result["max_velocity"]))
	if not accepted:
		_show_error(str(result.get("error", "The simulation returned an unspecified error.")))
	_update_labels()
	if _endurance != null and _endurance.native_active:
		_native_configuration_changed()
	return accepted


func _valid_selection(result: Dictionary) -> bool:
	if not result.get("genotype_preview") is Dictionary or not result.get("genotype_preview_error") is String:
		return false
	if not GenomePreview.valid_raster(result["genotype_preview"]):
		return false
	for field: String in ["selected", "selected_is_tracked", "tracking", "tracking_center", "tracking_path", "max_velocity", "methane"]:
		if not result.has(field):
			return false
	if not result["selected"] is Dictionary or not result["tracking_center"] is PackedFloat64Array:
		return false
	if not result["tracking_path"] is PackedInt32Array or result["tracking_path"].size() % 2 != 0:
		return false
	var center: PackedFloat64Array = result["tracking_center"]
	if center.size() != (2 if bool(result["tracking"]) else 0):
		return false
	var selected: Dictionary = result["selected"]
	if selected.is_empty():
		return not bool(result["selected_is_tracked"])
	for field: String in ["id", "age", "age_ticks", "maximum_age", "children", "kills", "infections", "reproduce_energy"]:
		if not selected.has(field) or not selected[field] is int:
			return false
	for field: String in ["energy", "mass"]:
		if not selected.has(field) or not selected[field] is float:
			return false
	for field: String in ["alive", "infected", "genotype_available"]:
		if not selected.has(field) or not selected[field] is bool:
			return false
	var display: Variant = selected.get("display")
	if not display is Dictionary:
		return false
	for field: String in ["id", "age", "maximum_age", "children", "kills", "infections", "reproduce_energy", "energy", "mass"]:
		if not display.get(field) is String:
			return false
	if bool(selected["genotype_available"]):
		for field: String in ["generation", "mutation_rate"]:
			if not selected.has(field) or not selected[field] is int or not display.get(field) is String:
				return false
		if not display.get("clade") is String:
			return false
		if not selected.has("clade") or not selected["clade"] is String:
			return false
	elif selected.has("generation") or selected.has("mutation_rate") or selected.has("clade") or display.has("generation") or display.has("mutation_rate") or display.has("clade"):
		return false
	return selected.has("bounds") and selected["bounds"] is PackedInt32Array and selected["bounds"].size() == 4


func _on_organism_picked(position: Vector2, context_menu: bool, viewport_position: Vector2) -> void:
	_manual_camera_override()
	_ambient_shell.reveal()
	if _settings_open() or not _preference_close_kind.is_empty():
		return
	_accrue_clock()
	_placement_position = position
	if not context_menu and not _placement_mode.is_empty():
		_place_at(position, _placement_mode)
		_set_placement_mode("")
		return
	var result: Dictionary = _simulation.select_at(position.x, position.y, context_menu)
	if _accept_result(result) and context_menu:
		_set_placement_mode("")
		var screen_position: Vector2 = viewport_position
		if not get_viewport().gui_embed_subwindows:
			screen_position += Vector2(get_window().position)
		if bool(result["selection_hit"]):
			_organism_controls.show_context(screen_position)
		else:
			_world_context.set_item_disabled(_world_context.get_item_index(4), not bool(_state["clipboard_ready"]))
			_world_context.position = Vector2i(screen_position)
			_world_context.popup()


func _on_organism_action(action: String) -> void:
	_accrue_clock()
	if _accept_result(_simulation.organism_action(action)):
		_error_label.visible = false
	get_viewport().gui_release_focus()


func _on_tracking_requested() -> void:
	_accrue_clock()
	if _accept_result(_simulation.track_selection()):
		_error_label.visible = false
	get_viewport().gui_release_focus()


func _on_abort_tracking_requested() -> void:
	if _accept_result(_simulation.abort_tracking()):
		_error_label.visible = false
	get_viewport().gui_release_focus()


func _on_world_action(command: int, pause_source: String = "menu") -> void:
	if command == 16:
		_toggle_pause(pause_source)
		return
	if _shutting_down or not _file_operation.is_empty() or not _decision_operation.is_empty() or not _preference_close_kind.is_empty() or _settings_open() or is_instance_valid(_about_window) or is_instance_valid(_version_window):
		return
	if command == 22:
		if not bool(get_node("/root/TouchKeyboard").call("use_app_keyboard")):
			_show_error("", "R_APP_KEYBOARD_ERROR")
		_refresh_main_actions()
		return
	match command:
		17:
			_request_create_world()
			return
		18:
			_quit()
			return
		19:
			_open_statistics()
			return
		20:
			_open_laboratory("clipboard")
			return
		21:
			_open_settings()
			return
	if _state.is_empty():
		return
	if command in [11, 12, 13, 14, 15]:
		if command == 15:
			_begin_file_operation("world_recover", Vector2.ZERO)
			return
		if command == 14 and not _world_path.is_empty():
			_accrue_clock()
			var result: Dictionary = _simulation.backup_world_file(_world_path)
			if bool(result["ok"]):
				_error_label.visible = false
			else:
				_show_error(str(result["error"]))
			_last_clock_usec = _clock_usec()
			get_viewport().gui_release_focus()
			return
		_begin_file_operation("world_open" if command == 11 else ("world_save" if command == 12 else "world_save_as"), Vector2.ZERO)
		return
	if command == 10:
		_on_abort_tracking_requested()
		return
	assert(command >= 4 and command <= 9, "Unknown world menu action")
	_accrue_clock()
	if _accept_result(_simulation.apply_commands(PackedInt32Array([command]))):
		_error_label.visible = false
	get_viewport().gui_release_focus()


func _settings_open() -> bool:
	return is_instance_valid(_settings_window) and _settings_window.visible


func _open_settings() -> void:
	if _network_open():
		_network_window.grab_focus()
		return
	if _shutting_down or not _decision_operation.is_empty() or not _file_operation.is_empty():
		return
	if is_instance_valid(_version_window):
		_version_window.grab_focus()
		return
	if not _preference_close_kind.is_empty():
		_preference_failure.grab_focus()
		return
	if _settings_open():
		_settings_window.grab_focus()
		return
	_accrue_clock()
	_settings_window = SettingsWindow.new()
	_settings_window.hide()
	_settings_window.name = "SettingsWindow"
	var parent_window: Window = get_window()
	var embed_settings: bool = parent_window.mode in [Window.MODE_FULLSCREEN, Window.MODE_EXCLUSIVE_FULLSCREEN]
	if embed_settings:
		_hide_settings_windows()
		if not parent_window.gui_embed_subwindows:
			parent_window.gui_embed_subwindows = true
			if not parent_window.gui_embed_subwindows:
				_settings_window.free()
				_settings_window = null
				_restore_settings_embedding()
				_show_error("Cannot display Parameters inside the fullscreen window")
				return
			_settings_embedding_owner = weakref(parent_window)
	_settings_window.force_native = not embed_settings
	if embed_settings:
		_settings_window.gui_embed_subwindows = true
	_settings_window.simulation = _simulation
	_settings_window.display_preferences = _display_preferences
	_settings_window.draft = _simulation.settings_open()
	_settings_window.locale_callback = _settings_locale_changed
	_settings_window.applied.connect(_settings_applied)
	_settings_window.finished.connect(_settings_finished)
	_settings_window.tree_exited.connect(_settings_removed)
	add_child(_settings_window)


func _update_world_shortcuts(language: String) -> void:
	_save_key = KEY_G if language == "es" else (KEY_D if language == "ca" else KEY_S)
	_backup_key = KEY_R if language == "es" else (KEY_K if language == "de" else (KEY_G if language == "ca" else KEY_B))
	_refresh_main_actions()


func _refresh_main_actions() -> void:
	_continuous_display_checkbox.text = MainLabels.message("R_CONTINUOUS_DISPLAY", _main_language)
	_continuous_display_help.text = MainLabels.message("R_CONTINUOUS_DISPLAY_HELP", _main_language)
	_continuous_display_checkbox.tooltip_text = _continuous_display_help.text
	_continuous_display_checkbox.accessibility_name = _continuous_display_checkbox.text
	_world_menu.text = MainLabels.message("T_GAME", _main_language)
	_actions_menu.text = MainLabels.message("T_GLOBAL_ACTIONS", _main_language)
	_world_tools_menu.text = MainLabels.message("T_WORLD", _main_language)
	_network_menu.text = MainLabels.message("T_NETWORK", _main_language)
	_network_button.text = MainLabels.message("T_MANAGE_CONNECTIONS", _main_language)
	_network_button.accessibility_name = _network_button.text
	_network_button.tooltip_text = _network_button.text
	for entry: Array in [[0, "T_CONFIGURE_NETWORK"], [1, "T_MANAGE_CONNECTIONS"]]:
		_network_menu.get_popup().set_item_text(_network_menu.get_popup().get_item_index(int(entry[0])), NetworkLabels.message(str(entry[1]), _main_language))
	if _network_open() and _network_window.language != _main_language:
		_network_window.apply_locale(_main_language)
	for reference: WeakRef in _network_managers.values():
		var manager: NetworkWindow = reference.get_ref() as NetworkWindow
		if manager != null and manager.language != _main_language:
			manager.apply_locale(_main_language)
	_refresh_network_status()
	_genetics_menu.text = MainLabels.message("R_GENETICS", _main_language)
	var tools_popup: PopupMenu = _world_tools_menu.get_popup()
	var keyboard_index: int = tools_popup.get_item_index(22)
	var system_keyboard: bool = FileAccess.file_exists("user://desktop-keyboard") and not bool(get_node("/root/TouchKeyboard").call("is_enabled"))
	if system_keyboard and keyboard_index == -1:
		tools_popup.add_item("", 22)
	elif not system_keyboard and keyboard_index != -1:
		tools_popup.remove_item(keyboard_index)
	if system_keyboard:
		tools_popup.set_item_text(tools_popup.get_item_index(22), MainLabels.message("R_USE_APP_KEYBOARD", _main_language))
	for menu: MenuButton in [_world_menu, _actions_menu, _world_tools_menu]:
		var popup: PopupMenu = menu.get_popup()
		for index: int in range(popup.get_item_count()):
			var command: int = popup.get_item_id(index)
			if MAIN_ACTION_LABELS.has(command):
				var labels: Array = MAIN_ACTION_LABELS[command]
				var caption: String = MainLabels.message(str(labels[0]), _main_language)
				if command == 12:
					caption += " [%s / Ctrl+S]" % OS.get_keycode_string(_save_key)
				elif command == 14:
					caption += " [%s]" % OS.get_keycode_string(_backup_key)
				popup.set_item_text(index, caption)
				popup.set_item_tooltip(index, MainLabels.message(str(labels[1]), _main_language))
	for index: int in range(_quick_buttons.size()):
		var labels: Array = MAIN_ACTION_LABELS[QUICK_ACTION_IDS[index]]
		_quick_buttons[index].accessibility_name = MainLabels.message(str(labels[0]), _main_language)
		_quick_buttons[index].tooltip_text = MainLabels.message(str(labels[1]), _main_language)
	for entry: Array in [["CreateWorldButton", 17], ["SettingsButton", 21], ["StatisticsButton", 19], ["QuitButton", 18]]:
		var button: Button = find_child(str(entry[0]), true, false) as Button
		var labels: Array = MAIN_ACTION_LABELS[int(entry[1])]
		button.text = MainLabels.message(str(labels[0]), _main_language)
		if int(entry[1]) == 18:
			button.text += " [Esc]"
		button.tooltip_text = MainLabels.message(str(labels[1]), _main_language)
		button.accessibility_name = MainLabels.message(str(labels[0]), _main_language)
	var reset_key: String = "Ctrl+R" if _backup_key == KEY_R else "R"
	_reset_button.text = MainLabels.message("R_RESET", _main_language) + " [%s]" % reset_key
	_reset_button.tooltip_text = MainLabels.message("R_RESET_PROCESS", _main_language) % reset_key
	_step_button.text = MainLabels.message("R_STEP", _main_language) + " [N]"
	for entry: Array in [["FitButton", "R_FIT", " [F]"], ["ZoomOutButton", "R_ZOOM_OUT", ""], ["ZoomInButton", "R_ZOOM_IN", ""]]:
		var button: Button = find_child(str(entry[0]), true, false) as Button
		button.text = MainLabels.message(str(entry[1]), _main_language) + str(entry[2])
	_crisp_lines.text = MainLabels.message("R_CRISP_LINES", _main_language)
	_crisp_lines.tooltip_text = MainLabels.message("R_CRISP_LINES_PROCESS", _main_language)
	_help_label.text = MainLabels.message("R_CONTROLS_HINT", _main_language) % [reset_key, OS.get_keycode_string(_save_key), OS.get_keycode_string(_backup_key)]
	var genetics: PopupMenu = _genetics_menu.get_popup()
	for entry: Array in [[0, "R_LAB_CLIPBOARD", "T_GENETIC_LABORATORY"], [1, "R_LAB_EMPTY", "T_GENETIC_LABORATORY"], [2, "T_COPY", "T_COPY_GENETIC_CODE"], [3, "R_EXAMINE_GENES", "R_EXAMINE_GENES"], [4, "T_PASTE", "T_PASTE_GENETIC_CODE"], [5, "T_CREATE_RANDOMLY", "T_CREATE_RANDOMLY"], [6, "T_IMPORT", "T_IMPORT_GENETIC_CODE"], [7, "T_EXPORT", "T_EXPORT_GENETIC_CODE"], [8, "T_SAVE_IMAGE", "T_SAVE_IMAGE"]]:
		var index: int = genetics.get_item_index(int(entry[0]))
		genetics.set_item_text(index, MainLabels.message(str(entry[1]), _main_language))
		genetics.set_item_tooltip(index, MainLabels.message(str(entry[2]), _main_language))
	for entry: Array in [[0, "T_GENETIC_LABORATORY"], [4, "T_PASTE"], [5, "T_CREATE_RANDOMLY"], [6, "T_IMPORT"]]:
		_world_context.set_item_text(_world_context.get_item_index(int(entry[0])), MainLabels.message(str(entry[1]), _main_language))
	_refresh_pause_action()


func _refresh_status_locale() -> void:
	_organism_controls.set_language(_main_language)
	_disclosure_label.text = MainLabels.message("R_DISCLOSURE", _main_language)
	var keys: Array[String] = ["R_SETUP_SEED", "R_SETUP_WIDTH", "R_SETUP_HEIGHT", "R_SETUP_ATTEMPTS"]
	var fields: Array[LineEdit] = [_seed_input, _width_input, _height_input, _attempts_input]
	for index: int in range(keys.size()):
		var caption: String = MainLabels.message(keys[index], _main_language)
		_setup_labels[index].text = caption
		fields[index].accessibility_name = caption
	_setup_labels[0].tooltip_text = MainLabels.message("R_CREATE_SETUP_HINT", _main_language)
	_refresh_placement_hint()
	if _error_label.visible:
		_refresh_feedback_locale()
	_refresh_modal_locale()
	_refresh_automatic_status()
	_on_view_changed(_view_zoom)
	_update_labels()


func _settings_locale_changed(payload: Dictionary) -> void:
	# This synchronous callback uses only the supplied pre-numeric presentation
	_main_language = str(payload["language"])
	_update_world_shortcuts(_main_language)
	_refresh_status_locale()
	if is_instance_valid(_settings_window):
		_settings_window.apply_locale(str(payload["language"]))
	_statistics_button.text = StatisticsLabels.message("T_STATISTICS", str(payload["language"]))
	_apply_help_locale(str(payload["language"]))
	language_changed.emit(_main_language)


func _apply_help_locale(language: String) -> void:
	_help_language = language
	_help_menu.text = HelpAbout.message("T_HELP", language)
	var popup: PopupMenu = _help_menu.get_popup()
	for entry: Array in [[0, "T_USER_MANUAL"], [2, "T_CHECK_LAST_VERSION"], [1, "T_ABOUT"]]:
		popup.set_item_text(popup.get_item_index(int(entry[0])), HelpAbout.message(str(entry[1]), language))



func _refresh_help_locale() -> void:
	if _simulation != null:
		_apply_help_locale(str(_simulation.settings_snapshot()["language"]))


func _on_world_popup_input(event: InputEvent) -> void:
	_route_main_popup_input(event, _world_menu.get_popup())


func _on_actions_popup_input(event: InputEvent) -> void:
	_route_main_popup_input(event, _actions_menu.get_popup())


func _on_world_tools_popup_input(event: InputEvent) -> void:
	_route_main_popup_input(event, _world_tools_menu.get_popup())


func _route_main_popup_input(event: InputEvent, popup: PopupMenu) -> void:
	if _network_open() or not popup.visible or not event is InputEventKey:
		return
	var key: InputEventKey = event as InputEventKey
	if not key.pressed or key.echo or key.ctrl_pressed or key.meta_pressed or key.shift_pressed:
		return
	if key.alt_pressed and _route_menu_heading_key(key, popup):
		return
	var matches: Array[int] = []
	for index: int in range(popup.get_item_count()):
		var command: int = popup.get_item_id(index)
		if popup.is_item_disabled(index):
			continue
		if command == 16:
			if key.keycode == _pause_action_key or (not key.alt_pressed and key.keycode == KEY_P):
				matches.append(index)
		elif MAIN_ACTION_LABELS.has(command) and command != 15:
			if key.keycode == MainLabels.mnemonic(str(MAIN_ACTION_LABELS[command][0]), _main_language):
				matches.append(index)
	if matches.is_empty():
		return
	popup.set_input_as_handled()
	if matches.size() > 1:
		var selected: int = popup.get_focused_item()
		var next_index: int = matches[0]
		for index: int in matches:
			if index > selected:
				next_index = index
				break
		popup.set_focused_item(next_index)
		return
	var command: int = popup.get_item_id(matches[0])
	popup.hide()
	_on_world_action(command)


func _route_menu_heading_key(key: InputEventKey, source_popup: PopupMenu = null) -> bool:
	if _network_open() or not key.alt_pressed or key.ctrl_pressed or key.meta_pressed or key.shift_pressed:
		return false
	if not get_window().has_focus() and (source_popup == null or not source_popup.has_focus()):
		return false
	var selected: MenuButton
	# Later Source headings win duplicate mnemonic keys
	for entry: Array in [[_world_menu, "T_GAME"], [_actions_menu, "T_GLOBAL_ACTIONS"], [_world_tools_menu, "T_WORLD"], [_network_menu, "T_NETWORK"], [_help_menu, "T_HELP"]]:
		var menu: MenuButton = entry[0] as MenuButton
		if not menu.disabled and key.keycode == MainLabels.mnemonic(str(entry[1]), _main_language):
			selected = menu
	if selected == null:
		return false
	get_viewport().set_input_as_handled()
	if source_popup != null:
		source_popup.set_input_as_handled()
		if source_popup == selected.get_popup():
			return true
		source_popup.hide()
	selected.show_popup()
	selected.get_popup().set_focused_item(0)
	return true


func _route_toolbar_mnemonic(key: InputEventKey) -> bool:
	if not key.alt_pressed or key.ctrl_pressed or key.meta_pressed or key.shift_pressed or not get_window().has_focus():
		return false
	var selected: int = 0
	for command: int in [17, 16, 12, 14, 6, 7, -1, 10]:
		var caption_key: String = "T_MANAGE_CONNECTIONS" if command == -1 else ("T_RESUME" if bool(_state.get("paused", true)) else "T_PAUSE")
		if command != -1 and command != 16:
			caption_key = str(MAIN_ACTION_LABELS[command][0])
		if key.keycode == MainLabels.mnemonic(caption_key, _main_language):
			selected = command
	if selected == 0:
		return false
	get_viewport().set_input_as_handled()
	if not get_viewport().gui_get_focus_owner() is LineEdit:
		if selected == -1:
			_open_network(1)
		elif selected != 10 or not _quick_buttons[4].disabled:
			_on_world_action(selected, "toolbar_mnemonic")
	return true


func _on_help_popup_input(event: InputEvent) -> void:
	if _network_open():
		return
	var popup: PopupMenu = _help_menu.get_popup()
	if not popup.visible or not event is InputEventKey:
		return
	var key: InputEventKey = event as InputEventKey
	if not key.pressed or key.echo or key.ctrl_pressed or key.meta_pressed:
		return
	if key.alt_pressed and _route_menu_heading_key(key, popup):
		return
	for entry: Array in [["T_USER_MANUAL_MNEMONIC", 0], ["T_CHECK_LAST_VERSION_MNEMONIC", 2], ["T_ABOUT_MNEMONIC", 1]]:
		if key.keycode == OS.find_keycode_from_string(HelpAbout.message(str(entry[0]), _help_language)):
			popup.set_input_as_handled()
			popup.hide()
			_on_help_action(int(entry[1]))
			return


func _on_help_action(action: int) -> void:
	if _shutting_down or _settings_open() or not _decision_operation.is_empty() or not _file_operation.is_empty():
		return
	if is_instance_valid(_version_window):
		_version_window.grab_focus()
		return
	var language: String = str(_simulation.settings_snapshot()["language"])
	_apply_help_locale(language)
	if action == 0:
		var target: String = HelpAbout.manual_target(language, HelpAbout.MANUAL_DIRECTORY)
		HelpAbout.open_manual(target, get_tree(), _manual_open_failed)
	elif action == 1:
		if is_instance_valid(_about_window):
			_about_window.grab_focus()
			return
		_about_window = HelpAbout.new()
		_about_window.name = "HelpAboutDialog"
		_about_window.language = language
		_about_window.theme = theme
		_about_window.closed.connect(_about_closed)
		add_child(_about_window)
	elif action == 2:
		if is_instance_valid(_about_window):
			_about_window.grab_focus()
			return
		_version_window = _new_version_check()
		_version_window.name = "HelpVersionCheckDialog"
		_version_window.language = language
		_version_window.language_provider = _version_language
		_version_window.theme = theme
		_version_window.closed.connect(_version_closed)
		add_child(_version_window)


func _new_version_check() -> VersionCheck:
	return VersionCheck.new()


func _version_language() -> String:
	return str(_simulation.settings_snapshot()["language"])


func _version_closed() -> void:
	_version_window = null
	if not _shutting_down:
		_help_menu.grab_focus()


func _cancel_version_check() -> void:
	if is_instance_valid(_version_window):
		_version_window.close_check()
	_version_window = null


func _manual_open_failed(message: String) -> void:
	if not _shutting_down and is_inside_tree():
		_show_error(message)


func _about_closed() -> void:
	_about_window = null
	if not _shutting_down:
		_help_menu.grab_focus()


func _open_statistics() -> void:
	if _shutting_down or _settings_open() or not _decision_operation.is_empty() or not _file_operation.is_empty():
		return
	_close_statistics()
	var result: Dictionary = _simulation.statistics_open()
	if not bool(result.get("ok", false)):
		_show_error(str(result.get("error", "Statistics are unavailable.")))
		return
	_statistics_window = StatisticsWindow.new()
	_statistics_window.hide()
	_statistics_window.name = "StatisticsWindow"
	_statistics_window.force_native = true
	_statistics_window.theme = theme
	_statistics_window.simulation = _simulation
	_statistics_window.initial_packet = result
	_statistics_window.closed.connect(_statistics_closed)
	_statistics_window.copied.connect(_statistics_copied)
	add_child(_statistics_window)


func _close_statistics() -> void:
	if is_instance_valid(_statistics_window):
		var window: StatisticsWindow = _statistics_window
		_statistics_window = null
		window.close_statistics()
	elif _simulation != null:
		_simulation.statistics_close()


func _statistics_closed() -> void:
	_statistics_window = null
	if not _shutting_down and is_instance_valid(_canvas):
		_canvas.grab_focus()


func _statistics_copied(result: Dictionary) -> void:
	if not _shutting_down and not _state.is_empty():
		_state["clipboard_ready"] = bool(result["clipboard_ready"])
		_update_labels()


func _settings_applied(result: Dictionary) -> void:
	_apply_cinema_preferences()
	_accrue_clock()
	_stored_delay_ms = int(result["values"]["DELAY"])
	_reset_pacing()
	var genesis: PackedInt64Array = result["genesis"]
	_width_input.text = str(genesis[0])
	_height_input.text = str(genesis[1])
	_attempts_input.text = str(genesis[2])
	_accept_result(_simulation.snapshot())
	var retained: PackedStringArray = result["retained"]
	if not retained.is_empty():
		_retained_input_count = retained.size()
		_refresh_feedback_locale()
		_error_label.show()
		_settings_report.text = "\n".join(retained)
		_settings_report.show()
	else:
		_error_label.hide()
		_settings_report.hide()
	_update_labels()
	_save_preferences_current_geometry()
	if not str(result.get("display_error", "")).is_empty():
		_show_error(str(result["display_error"]))


func _update_automatic_status(status: Dictionary) -> void:
	_automatic_status = status
	_refresh_automatic_status()


func _refresh_automatic_status() -> void:
	var status: Dictionary = _automatic_status
	if status.is_empty():
		return
	var managed: bool = str(status["mode"]) == "managed"
	var manual_pending: bool = bool(status.get("manual_checkpoint_pending", false))
	var text: String = MainLabels.message("R_AUTO_MANAGED" if managed else "R_AUTO_ADJACENT", _main_language)
	if not bool(status["enabled"]):
		text = MainLabels.message("R_AUTO_OFF", _main_language)
	var last: Dictionary = status["last"]
	if not last.is_empty():
		text += " " + MainLabels.message("R_AUTO_LAST_TIME", _main_language) % str(int(last["time"]))
	if bool(status["source_save_pending"]):
		text += " " + MainLabels.message("R_AUTO_SAVE_PENDING", _main_language)
	if bool(status.get("checkpoint_pending", false)):
		text += " " + MainLabels.message("R_AUTO_CHECKPOINT_PENDING", _main_language)
	if not str(status["error"]).is_empty():
		text += " " + str(status["error"])
	if bool(status["replacement_committed"]):
		text += " " + MainLabels.message("R_AUTO_DURABILITY", _main_language)
	if not str(status.get("recovery_warning", "")).is_empty():
		text += " " + MainLabels.message("R_AUTO_RECOVERY_WARNING", _main_language) % str(status["recovery_warning"])
	if bool(status["enabled"]) and not managed and _is_timestamped_backup(_world_path):
		text += " " + MainLabels.message("R_AUTO_TIMESTAMP_WARNING", _main_language)
	if manual_pending:
		text += " " + MainLabels.message("R_MANUAL_PENDING", _main_language)
	if not str(status.get("manual_checkpoint_warning", "")).is_empty():
		text += " " + str(status["manual_checkpoint_warning"])
	_automatic_label.text = text
	_automatic_label.tooltip_text = text + "\n" + MainLabels.message("R_AUTO_DIRECTORY", _main_language) % str(status["directory"])
	_automatic_label.visible = manual_pending or bool(status.get("checkpoint_pending", false)) or not str(status.get("manual_checkpoint_warning", "")).is_empty() or bool(status["enabled"]) or not str(status["error"]).is_empty() or not last.is_empty() or not str(status.get("recovery_warning", "")).is_empty()


func _is_timestamped_backup(path: String) -> bool:
	if path.get_extension().to_lower() != "bgw":
		return false
	var base: String = path.get_file().get_basename()
	if not "@" in base:
		return false
	var stamp: String = base.get_slice("@", base.get_slice_count("@") - 1)
	for index: int in range(stamp.length()):
		if stamp.unicode_at(index) < 48 or stamp.unicode_at(index) > 57:
			return false
	return true


func _refresh_automatic_context() -> void:
	_update_automatic_status(_simulation.refresh_automatic_backups(_world_path))


func _consume_source_backup() -> void:
	if not bool(_automatic_status.get("source_save_pending", false)) or _automatic_source_active or _shutting_down:
		return
	if not _file_operation.is_empty() or not _decision_operation.is_empty() or not _preference_close_kind.is_empty() or _network_open() or _settings_open() or is_instance_valid(_version_window) or is_instance_valid(_about_window):
		return
	_automatic_source_active = true
	if _world_path.is_empty():
		_begin_file_operation("world_save_as", Vector2.ZERO)
		if _file_operation.is_empty():
			_update_automatic_status(_simulation.complete_automatic_backup(false, "Source Backup Save As could not start", false, _world_path))
			_automatic_source_active = false
	else:
		var result: Dictionary = _simulation.backup_world_file(_world_path)
		_update_automatic_status(_simulation.complete_automatic_backup(bool(result["ok"]), str(result["error"]), bool(result.get("replacement_committed", false)), _world_path))
		_automatic_source_active = false


func _settings_finished() -> void:
	_accrue_clock()


func _hide_settings_windows() -> void:
	for child: Node in find_children("*", "Window", true, false):
		var window: Window = child as Window
		if window.visible:
			window.hide()
			if not window is Popup:
				_settings_hidden_windows.append(window)


func _restore_settings_embedding() -> void:
	if _settings_embedding_owner != null:
		var parent_window: Window = _settings_embedding_owner.get_ref() as Window
		_settings_embedding_owner = null
		if is_instance_valid(parent_window):
			parent_window.gui_embed_subwindows = false
			if parent_window.gui_embed_subwindows:
				push_error("Cannot restore Parameters window embedding")
	if not _shutting_down:
		for window: Window in _settings_hidden_windows:
			if is_instance_valid(window) and window.is_inside_tree() and not window.is_queued_for_deletion():
				window.show()
	_settings_hidden_windows.clear()


func _settings_removed() -> void:
	_settings_window = null
	_restore_settings_embedding()
	if not _shutting_down and is_instance_valid(_canvas):
		_canvas.grab_focus()


func _refresh_feedback_locale() -> void:
	if _retained_input_count > 0:
		_error_label.text = MainLabels.message("R_SETTINGS_RETAINED", _main_language) % _retained_input_count
	else:
		var message: String = MainLabels.message(_error_message_key, _main_language) if not _error_message_key.is_empty() else _error_message
		if not _network_error_packet.is_empty():
			message = NetworkLabels.feedback(_network_error_packet, "error", _main_language)
		_error_label.text = MainLabels.message("R_ERROR", _main_language) % message


func _show_error(message: String, message_key: String = "", network_packet: Dictionary = {}) -> void:
	_retained_input_count = 0
	_error_message = message
	_error_message_key = message_key
	_network_error_packet = {} if network_packet.is_empty() else {
		"error": network_packet.get("error", ""),
		"error_feedback": network_packet.get("error_feedback")
	}
	_refresh_feedback_locale()
	_error_label.visible = true
	if is_instance_valid(_ambient_shell):
		_ambient_shell.show_tools()
	if _state.is_empty():
		_mode_label.text = MainLabels.message("R_INITIALIZATION_ERROR", _main_language)
		_update_labels()


func _clock_usec() -> int:
	return Time.get_ticks_usec()


func _accrue_clock() -> void:
	var now: int = _clock_usec()
	if _last_clock_usec > 0 and not _state.is_empty() and not bool(_state["paused"]) and not _runtime_failed:
		_pending_ticks += float(now - _last_clock_usec) / (1000.0 * _pacing.current_delay_ms)
	_last_clock_usec = now
	if _endurance != null:
		_endurance.observe_debt(_pending_ticks)


func _process(_delta: float) -> void:
	if is_instance_valid(_ambient_shell):
		_ambient_shell.hold_open = _presentation_busy() or _error_label.visible
		# Engine delta can be capped under load; UI deadlines use monotonic time
		var now: int = _clock_usec()
		var elapsed: float = float(maxi(0, now - _presentation_clock_usec)) / 1000000.0
		_presentation_clock_usec = now
		_update_cinema(elapsed)
	if _endurance != null and _endurance.native_enabled:
		if _endurance.native_collecting():
			_native_configuration_changed()
		_endurance.native_flush_pending()
	if _endurance != null:
		_observe_endurance()
		_canvas.measure_submission = _endurance.active
	if _shutting_down:
		return
	if bool(_automatic_status.get("checkpoint_pending", false)):
		_update_automatic_status(_simulation.automatic_backup_status())
	_pump_network()
	_accrue_clock()
	if bool(_automatic_status.get("source_save_pending", false)):
		_consume_source_backup()
		return
	if _state.is_empty() or bool(_state["paused"]) or _runtime_failed:
		return
	var due: int = mini(int(_pending_ticks), MAX_TICKS_PER_FRAME)
	if due > 0:
		var before_tick: int = int(_state["tick"])
		var started: int = _clock_usec()
		var accepted: bool = _accept_result(_simulation.advance_budgeted(due, SIMULATION_BUDGET_USEC))
		if not accepted:
			if _endurance != null:
				_endurance.observe_batch(maxi(0, int(_state["tick"]) - before_tick), _clock_usec() - started, false)
			_runtime_failed = true
			_update_labels()
			return
		var finished: int = _clock_usec()
		_last_step_ms = float(finished - started) / 1000.0
		var completed: int = int(_state["tick"]) - before_tick
		if _endurance != null:
			_endurance.observe_batch(completed, finished - started, true)
		_pending_ticks = maxf(0.0, _pending_ticks - completed)
		_observe_pacing(completed, _clock_usec())
		_update_labels()
		_consume_source_backup()


func _reset_pacing() -> void:
	_pacing.reset(maxi(1, _stored_delay_ms))
	_pacing_sample_usec = 0
	_overload_paused = false


func _observe_pacing(completed: int, now_usec: int) -> void:
	if completed <= 0 or bool(_state["paused"]):
		_pacing.clear_continuous_sample()
		_pacing_sample_usec = 0
		return
	_accrue_clock()
	if _pacing_sample_usec <= 0:
		_pacing_sample_usec = now_usec
		return
	var elapsed_usec: int = maxi(0, now_usec - _pacing_sample_usec)
	_pacing_sample_usec = now_usec
	if _continuous_display:
		_pacing.observe_continuous(elapsed_usec, completed, maxi(1, _stored_delay_ms), _pending_ticks >= 1.0)
		return
	# Normalize the wall interval by completed biological ticks rather than rendered batches
	var elapsed_ms: float = float(elapsed_usec) / (1000.0 * completed)
	var previous_delay: int = _pacing.current_delay_ms
	_pacing.observe(elapsed_ms, maxi(1, _stored_delay_ms))
	if previous_delay == 1 and _pacing.current_delay_ms == 1 and _pacing.restarted:
		# Source integer truncation must not prevent native overload recovery
		_pacing.current_delay_ms = 2
	if _pacing.pause_requested and _pacing.current_delay_ms > maxi(1, _stored_delay_ms):
		_overload_paused = true
		if _endurance != null and _endurance.native_active:
			_endurance.native_interruption("overload", _clock_usec())
		if not _accept_result(_simulation.apply_commands(PackedInt32Array([0]))):
			_runtime_failed = true


func _toggle_pause(source: String = "toolbar") -> void:
	if _state.is_empty() or _runtime_failed or _shutting_down or _network_open() or _settings_open() or not _preference_close_kind.is_empty() or not _decision_operation.is_empty() or not _file_operation.is_empty() or is_instance_valid(_about_window) or is_instance_valid(_version_window):
		return
	_accrue_clock()
	var operation: int = 1 if bool(_state["paused"]) else 0
	var request: int = 0
	if _endurance != null and _endurance.native_active:
		request = _endurance.native_request(source, _clock_usec(), int(_state["tick"]), bool(_state["paused"]))
	var accepted: bool = _accept_result(_simulation.apply_commands(PackedInt32Array([operation])))
	if _endurance != null and _endurance.native_active:
		_endurance.native_command(request, _clock_usec(), accepted, int(_state["tick"]), bool(_state["paused"]))
	get_viewport().gui_release_focus()


func _single_step() -> void:
	if _state.is_empty() or not bool(_state["paused"]) or _runtime_failed:
		return
	_accrue_clock()
	if not _accept_result(_simulation.apply_commands(PackedInt32Array([2]))):
		_runtime_failed = true
		_update_labels()
	get_viewport().gui_release_focus()
	_consume_source_backup()


func _reset_world() -> void:
	if _state.is_empty():
		return
	_request_world_decision("reset")


func _apply_reset() -> bool:
	_accrue_clock()
	if _accept_result(_simulation.apply_commands(PackedInt32Array([3]))):
		_network_world_replaced()
		_pending_ticks = 0.0
		_reset_pacing()
		_runtime_failed = false
		_error_label.visible = false
		_last_clock_usec = _clock_usec()
		_update_labels()
		return true
	return false


func _refresh_pause_action() -> void:
	var paused: bool = bool(_state.get("paused", true))
	var action: String = "T_RESUME" if paused else "T_PAUSE"
	var caption: String = MainLabels.message(action, _main_language)
	var description: String = MainLabels.message(action + "_PROCESS", _main_language)
	_resume_key = OS.find_keycode_from_string(MainLabels.message("T_RESUME_MNEMONIC", _main_language))
	_pause_key = OS.find_keycode_from_string(MainLabels.message("T_PAUSE_MNEMONIC", _main_language))
	_pause_action_key = _resume_key if paused else _pause_key
	_pause_button.text = caption + " [P / Space]"
	_pause_button.tooltip_text = description
	_pause_button.icon = RESUME_ICON if paused else PAUSE_ICON
	var unavailable: bool = _state.is_empty() or _runtime_failed or _shutting_down
	_pause_button.disabled = unavailable
	var popup: PopupMenu = _world_menu.get_popup()
	var index: int = popup.get_item_index(16)
	popup.set_item_text(index, caption)
	popup.set_item_tooltip(index, description)
	popup.set_item_disabled(index, unavailable)
	if _state.is_empty():
		_mode_label.text = MainLabels.message("R_INITIALIZATION_ERROR" if _error_label.visible else "R_INITIALIZING", _main_language)
	else:
		_mode_label.text = MainLabels.message("R_STOPPED_ERROR", _main_language) if _runtime_failed else MainLabels.message("T_GAME_PAUSED" if paused else "T_GAME_RESUMED", _main_language)


func _refresh_main_action_availability() -> void:
	var unavailable: bool = _state.is_empty() or _shutting_down
	for menu: MenuButton in [_world_menu, _actions_menu, _world_tools_menu]:
		menu.disabled = unavailable
	for button: Button in _quick_buttons:
		button.disabled = unavailable
	var cannot_abort: bool = unavailable or not bool(_state.get("tracking", false))
	_quick_buttons[4].disabled = cannot_abort
	_world_tools_menu.get_popup().set_item_disabled(_world_tools_menu.get_popup().get_item_index(10), cannot_abort)


func _update_labels() -> void:
	_refresh_pause_action()
	_refresh_main_action_availability()
	if _state.is_empty():
		_pause_button.disabled = true
		_step_button.disabled = true
		_reset_button.disabled = true
		return
	var paused: bool = bool(_state["paused"])
	if is_instance_valid(_ambient_shell):
		_ambient_shell.set_paused(paused)
		_ambient_shell.set_language(_main_language)
		_refresh_fullscreen_caption()
	_seed_input.tooltip_text = MainLabels.message("R_SEED_HINT", _main_language) % str(_state["seed"])
	_width_input.tooltip_text = MainLabels.message("R_WIDTH_HINT", _main_language) % int(_state["width"])
	_height_input.tooltip_text = MainLabels.message("R_HEIGHT_HINT", _main_language) % int(_state["height"])
	_attempts_input.tooltip_text = MainLabels.message("R_ATTEMPTS_HINT", _main_language) % int(_state["initial_attempts"])
	var genome_menu: PopupMenu = _genetics_menu.get_popup()
	var selected: Dictionary = _state["selected"]
	var alive: bool = not selected.is_empty() and bool(selected["alive"])
	genome_menu.set_item_disabled(genome_menu.get_item_index(2), not alive)
	genome_menu.set_item_disabled(genome_menu.get_item_index(3), selected.is_empty())
	genome_menu.set_item_disabled(genome_menu.get_item_index(4), not bool(_state["clipboard_ready"]))
	genome_menu.set_item_disabled(genome_menu.get_item_index(7), not alive or not bool(selected["genotype_available"]))
	_step_button.disabled = not paused or _runtime_failed
	_reset_button.disabled = false
	_metrics_label.text = "   ".join(PackedStringArray([
		MainLabels.message("T_TIME", _main_language) + str(int(_state["world_time"])),
		MainLabels.message("R_TICK", _main_language) + str(int(_state["tick"])),
		MainLabels.message("T_CURRENT_POPULATION", _main_language) + str(int(_state["population"])),
		MainLabels.message("T_O2", _main_language) + MainLabels.format_number(float(_state["oxygen"]), _main_language),
		MainLabels.message("T_CO2", _main_language) + MainLabels.format_number(float(_state["carbon_dioxide"]), _main_language),
		MainLabels.message("T_CH4", _main_language) + MainLabels.format_number(float(_state["methane"]), _main_language)
	]))
	var pending: int = int(_pending_ticks)
	_timing_label.text = MainLabels.message("R_TIMING", _main_language) % [_stored_delay_ms, MainLabels.format_number(_last_step_ms, _main_language), pending]
	_timing_label.text += "   " + MainLabels.message("R_TARGET_TICKS", _main_language) % MainLabels.format_number(1000.0 / maxi(1, _stored_delay_ms), _main_language)
	if _stored_delay_ms <= 0:
		_timing_label.text += "\n" + MainLabels.message("R_SAFETY_DELAY", _main_language)
	if _overload_paused:
		_timing_label.text = MainLabels.message("R_OVERLOAD_PAUSED", _main_language) % pending
	elif _pacing.current_delay_ms != maxi(1, _stored_delay_ms):
		_timing_label.text += "   " + MainLabels.message("R_ADAPTIVE_TIMING", _main_language) % [_pacing.current_delay_ms, MainLabels.format_number(1000.0 / _pacing.current_delay_ms, _main_language)]
	elif pending > MAX_TICKS_PER_FRAME:
		_timing_label.text += "   " + MainLabels.message("R_BEHIND_SPEED", _main_language)
	_timing_label.add_theme_color_override("font_color", Color("e4bd7c") if _overload_paused or pending > MAX_TICKS_PER_FRAME else Color("9eb4c3"))


func _on_view_changed(zoom: float) -> void:
	_view_zoom = zoom
	if is_instance_valid(_zoom_label):
		_zoom_label.text = MainLabels.message("R_ZOOM", _main_language) % MainLabels.format_number(zoom * 100.0, _main_language, 1)


func _fit_world() -> void:
	_manual_camera_override()
	_canvas.fit_world()


func _presentation_busy() -> bool:
	return _shutting_down or _network_open() or _settings_open() or not _file_operation.is_empty() \
		or not _decision_operation.is_empty() or not _recovery_choice.is_empty() or not _preference_close_kind.is_empty() \
		or _organism_controls.has_open_popup() or _world_context.visible \
		or _world_menu.get_popup().visible or _actions_menu.get_popup().visible \
		or _world_tools_menu.get_popup().visible or _network_menu.get_popup().visible \
		or _genetics_menu.get_popup().visible or _help_menu.get_popup().visible \
		or is_instance_valid(_about_window) or is_instance_valid(_version_window)


func _apply_display_mode() -> void:
	if DisplayServer.get_name() == "headless":
		if startup_resume_managed:
			_resume_after_startup()
		return
	var window: Window = get_window()
	_windowed_geometry = Rect2i(window.position, window.size)
	_windowed_mode = window.mode
	if bool(_display_preferences.values["fullscreen"]):
		window.mode = Window.MODE_FULLSCREEN
	_refresh_fullscreen_caption.call_deferred()
	_resume_after_startup()


func _resume_after_startup() -> void:
	if startup_resume_managed and not _startup_recovery_succeeded:
		return
	if (_cinema_enabled or startup_resume_managed) and not _state.is_empty() and bool(_state["paused"]) and not _presentation_busy() and not _runtime_failed:
		_toggle_pause("living display startup")


func _toggle_fullscreen() -> void:
	var window: Window = get_window()
	var was_fullscreen: bool = window.mode in [Window.MODE_FULLSCREEN, Window.MODE_EXCLUSIVE_FULLSCREEN]
	if was_fullscreen:
		window.mode = Window.MODE_WINDOWED
		if _windowed_geometry.size.x > 0 and _windowed_geometry.size.y > 0:
			window.size = _windowed_geometry.size
			window.position = _windowed_geometry.position
		if _windowed_mode == Window.MODE_MAXIMIZED:
			window.mode = Window.MODE_MAXIMIZED
	else:
		_windowed_geometry = Rect2i(window.position, window.size)
		_windowed_mode = window.mode
		window.mode = Window.MODE_FULLSCREEN
	_display_preferences.values["fullscreen"] = not was_fullscreen
	_save_display_preferences()
	_ambient_shell.reveal()
	_refresh_fullscreen_caption.call_deferred()


func _refresh_fullscreen_caption() -> void:
	if not is_instance_valid(_fullscreen_button):
		return
	var captions: Array = FULLSCREEN_LABELS.get(_main_language, FULLSCREEN_LABELS["en"])
	_fullscreen_button.text = str(captions[1 if get_window().mode in [Window.MODE_FULLSCREEN, Window.MODE_EXCLUSIVE_FULLSCREEN] else 0])


func _save_display_preferences() -> void:
	var saved: Dictionary = _display_preferences.save_settings()
	if not bool(saved["ok"]):
		_show_error(str(saved["error"]))


func _set_continuous_display(enabled: bool) -> void:
	if enabled != _continuous_display:
		_pacing.clear_continuous_sample()
		_pacing.failed_time = 0
		_pacing_sample_usec = 0
	_continuous_display = enabled
	_display_preferences.values["continuous_display"] = enabled
	_save_display_preferences()


func _set_cinema_enabled(enabled: bool) -> void:
	_cinema_enabled = enabled
	_manual_camera_until = 0.0
	_director.cancel_action()
	_living_cards.dismiss()
	_display_preferences.values["cinema_enabled"] = enabled
	_save_display_preferences()


func _apply_cinema_preferences() -> void:
	_director.motion_rate_scale = float(_display_preferences.values["camera_motion_percent"]) / 100.0
	_director.settled_hold_seconds = float(_display_preferences.values["camera_settled_hold_seconds"])


func _manual_camera_override() -> void:
	_manual_camera_until = float(_clock_usec()) / 1000000.0 + 2.0 * float(_display_preferences.values["camera_hold_seconds"])
	_director.clear()
	_living_cards.dismiss()


func _cinema_allowed() -> bool:
	return _cinema_enabled and _ambient_shell.is_idle() and not _state.is_empty() and not bool(_state["paused"]) \
		and not bool(_state.get("tracking", false)) and not _presentation_busy() \
		and not _runtime_failed and not _error_label.visible \
		and float(_clock_usec()) / 1000000.0 >= _manual_camera_until


func _update_cinema(delta: float) -> void:
	if not is_finite(delta) or delta <= 0.0:
		return
	var allowed: bool = _cinema_allowed()
	_living_cards.set_avoid_rect(_ambient_shell.get_inspector_rect())
	_living_cards.advance(delta, allowed and _ambient_shell.is_idle())
	if not allowed:
		_director.cancel_action()
		_scene_poll_elapsed = 1.0
		return
	var elapsed: float = clampf(delta, 0.0, 0.25)
	_scene_poll_elapsed += delta
	var all_time_requested: bool = _living_cards.needs_all_time_records()
	if (_scene_poll_elapsed >= 0.5 or _living_cards.needs_previews() or all_time_requested) and _simulation.has_method("living_scene_snapshot"):
		_scene_poll_elapsed = 0.0
		var snapshot_method: StringName = &"living_scene_snapshot_with_records" if all_time_requested else &"living_scene_snapshot"
		if not _simulation.has_method(snapshot_method):
			_director.clear()
			_living_cards.dismiss()
			_show_error("The simulation extension does not support all-time record slides")
			return
		var packet: Dictionary = _simulation.call(snapshot_method, _director.follow_token, _living_cards.needs_previews())
		if not bool(packet.get("ok", false)):
			_director.clear()
			_living_cards.dismiss()
			_show_error(str(packet.get("error", "Living scene data is unavailable")))
			return
		_director.set_scene(packet)
		_living_cards.set_scene(packet, {
			"population": int(_state["population"]), "world_time": int(_state["world_time"]),
			"language": _main_language, "gases": {"oxygen": _state["oxygen"],
			"carbon_dioxide": _state["carbon_dioxide"], "methane": _state["methane"]}
		})
	var world: Vector2 = Vector2(float(_state["width"]), float(_state["height"]))
	var view: Dictionary = _canvas.view_state()
	var target: Dictionary = _director.advance(delta, world, _canvas.size, _living_cards.current_subject_token(), view)
	if target.is_empty():
		return
	_cinema_kind = target["kind"]
	var blended: Dictionary = CinemaDirector.blend_view(view, target, world, _canvas.size, elapsed, _director.motion_rate_scale)
	_canvas.set_camera_view(blended["center"], clampf(float(blended["zoom"]), WorldCanvas.MIN_ZOOM, WorldCanvas.MAX_ZOOM))


func _zoom_in() -> void:
	_manual_camera_override()
	_canvas.zoom_at(1.25, _canvas.size * 0.5)


func _zoom_out() -> void:
	_manual_camera_override()
	_canvas.zoom_at(0.8, _canvas.size * 0.5)


func _input(event: InputEvent) -> void:
	if _network_open() or _settings_open() or not _preference_close_kind.is_empty():
		return
	if _shutting_down or not _decision_operation.is_empty() or not _recovery_choice.is_empty():
		return
	if _organism_controls.has_open_popup() or _world_menu.get_popup().visible or _actions_menu.get_popup().visible or _world_tools_menu.get_popup().visible or _network_menu.get_popup().visible or _genetics_menu.get_popup().visible or _help_menu.get_popup().visible or is_instance_valid(_about_window) or is_instance_valid(_version_window) or _world_context.visible or not _file_operation.is_empty():
		return
	if not event is InputEventKey:
		return
	var key: InputEventKey = event as InputEventKey
	if not key.pressed or key.echo:
		return
	if key.physical_keycode == KEY_F11 and not key.ctrl_pressed and not key.alt_pressed and not key.meta_pressed:
		_toggle_fullscreen()
		get_viewport().set_input_as_handled()
		return
	if key.alt_pressed:
		_ambient_shell.show_tools()
	if _route_menu_heading_key(key):
		return
	if key.ctrl_pressed and key.physical_keycode == KEY_S and not (key.alt_pressed and key.keycode in [_resume_key, _pause_key]):
		_on_world_action(12)
		get_viewport().set_input_as_handled()
		return
	if _route_toolbar_mnemonic(key):
		return
	var focused: Control = get_viewport().gui_get_focus_owner()
	if focused is LineEdit and key.physical_keycode != KEY_ESCAPE:
		return
	if focused is BaseButton and key.is_action("ui_accept"):
		return
	if key.alt_pressed or key.meta_pressed or key.shift_pressed:
		return
	if not key.ctrl_pressed and not key.alt_pressed and not key.meta_pressed and not key.shift_pressed:
		if key.physical_keycode == KEY_P:
			_toggle_pause("p")
			get_viewport().set_input_as_handled()
			return
		if key.physical_keycode == _save_key:
			_on_world_action(12)
			get_viewport().set_input_as_handled()
			return
	if _backup_key == KEY_R and key.physical_keycode == KEY_R and key.ctrl_pressed:
		_reset_world()
		get_viewport().set_input_as_handled()
		return
	if key.physical_keycode == _backup_key and not key.ctrl_pressed and not key.alt_pressed and not key.meta_pressed and not key.shift_pressed:
		_on_world_action(14)
		get_viewport().set_input_as_handled()
		return
	match key.physical_keycode:
		KEY_SPACE:
			_toggle_pause("space")
		KEY_N:
			_single_step()
		KEY_R:
			_reset_world()
		KEY_F:
			_fit_world()
		KEY_EQUAL, KEY_PLUS, KEY_KP_ADD:
			_zoom_in()
		KEY_MINUS, KEY_KP_SUBTRACT:
			_zoom_out()
		KEY_ESCAPE:
			if not _placement_mode.is_empty():
				_set_placement_mode("")
			else:
				_quit()
		KEY_Q:
			_quit()
		_:
			return
	get_viewport().set_input_as_handled()


func _quit() -> void:
	_request_world_decision("quit")


func request_close() -> bool:
	return _request_world_decision("quit")


func _refresh_modal_locale() -> void:
	_recovery_dialog.title = MainLabels.message("R_MANUAL_RECOVERY_TITLE", _main_language)
	_recovery_dialog.dialog_text = MainLabels.message("R_FILE_ONLY_BODY" if _recovery_choice == "file_only" else "R_MANUAL_RECOVERY_BODY", _main_language)
	if not _recovery_detail.is_empty():
		_recovery_dialog.dialog_text += "\n" + _recovery_detail
	_recovery_dialog.ok_button_text = MainLabels.message("R_SAVE_FILE_ONLY" if _recovery_choice == "file_only" else "R_RECOVER_OLDER", _main_language)
	_recovery_dialog.get_cancel_button().text = MainLabels.message("T_CANCEL", _main_language)
	_open_manual_button.text = MainLabels.message("T_OPEN_WORLD", _main_language)
	_open_manual_button.visible = _recovery_choice == "older"
	var closing: bool = _decision_operation == "quit"
	if _creating_world:
		_decision_dialog.title = MainLabels.message("T_CREATE_NEW_WORLD", _main_language)
		_decision_dialog.dialog_text = MainLabels.message("R_INITIALIZING", _main_language)
	else:
		_decision_dialog.title = MainLabels.message("T_SAVE_WORLD" if closing else "R_REPLACE_WORLD_TITLE", _main_language)
		_decision_dialog.dialog_text = MainLabels.message("T_SAVE_BEFORE_QUIT" if closing else "R_REPLACE_WORLD_BODY", _main_language)
	_decision_dialog.ok_button_text = MainLabels.message("R_DISCARD_AND_CLOSE" if closing else ("T_CREATE_NEW_WORLD" if _decision_operation == "create" else "R_RESET"), _main_language)
	_decision_dialog.get_cancel_button().text = MainLabels.message("T_CANCEL", _main_language)
	_save_world_button.text = MainLabels.message("T_SAVE_WORLD", _main_language)
	_preference_failure.title = MainLabels.message("R_PREFERENCES_SAVE_FAILED", _main_language)
	_preference_failure.dialog_text = MainLabels.message("R_ERROR", _main_language) % _preference_failure_detail + "\n" + MainLabels.message("R_PREFERENCES_SAVE_HELP", _main_language)
	_preference_failure.ok_button_text = MainLabels.message("R_RETRY_SAVE", _main_language)
	_preference_failure.get_cancel_button().text = MainLabels.message("T_CANCEL", _main_language)
	_discard_preferences_button.text = MainLabels.message("R_CONTINUE_WITHOUT_SAVING", _main_language)
	_overwrite_dialog.title = MainLabels.message("T_FILE_EXISTS", _main_language)
	_overwrite_dialog.dialog_text = MainLabels.message("T_CONFIRM_FILE_OVERRIDE", _main_language)
	if not _save_path.is_empty():
		_overwrite_dialog.dialog_text += "\n" + _save_path.get_file()
	_overwrite_dialog.ok_button_text = MainLabels.message("T_YES", _main_language)
	_overwrite_dialog.get_cancel_button().text = MainLabels.message("T_CANCEL", _main_language)


func _request_world_decision(operation: String, values: PackedInt64Array = PackedInt64Array()) -> bool:
	if _shutting_down or not _file_operation.is_empty() or not _recovery_choice.is_empty() or not _preference_close_kind.is_empty():
		return false
	if not _decision_operation.is_empty():
		return operation == "quit" and _decision_operation == "quit"
	_cancel_version_check()
	_close_network_window()
	_accrue_clock()
	_decision_restore_running = not _state.is_empty() and not bool(_state["paused"])
	if _endurance != null and _endurance.native_active and _decision_restore_running:
		_endurance.native_interruption("quit" if operation == "quit" else "world_decision_" + operation, _clock_usec())
	if _decision_restore_running and not _accept_result(_simulation.apply_commands(PackedInt32Array([0]))):
		return false
	_decision_operation = operation
	_decision_values = values.duplicate()
	_decision_dialog.dialog_hide_on_ok = operation != "create"
	_refresh_modal_locale()
	if _settings_open():
		_settings_window.hide()
	_decision_dialog.popup_centered_clamped(Vector2i(560, 180))
	_decision_dialog.get_cancel_button().grab_focus()
	return true


func _confirm_world_decision() -> void:
	var operation: String = _decision_operation
	if operation.is_empty() or _awaiting_world_save or _creating_world:
		return
	if operation == "quit":
		_finish_preference_close("quit")
		return
	var succeeded: bool
	if operation == "create":
		var values: PackedInt64Array = _decision_values.duplicate()
		_creating_world = true
		_creation_generation += 1
		var generation: int = _creation_generation
		_decision_dialog.get_ok_button().disabled = true
		_decision_dialog.get_cancel_button().disabled = true
		_save_world_button.disabled = true
		_refresh_modal_locale()
		await RenderingServer.frame_post_draw
		if not is_inside_tree() or _shutting_down or not _creating_world or _creation_generation != generation or _decision_operation != operation:
			return
		succeeded = _configure_world(values)
	else:
		succeeded = _apply_reset()
	_end_world_decision(not succeeded)


func _cancel_world_decision() -> void:
	var closing: bool = _decision_operation == "quit"
	_end_world_decision(true)
	if closing:
		close_cancelled.emit()


func _world_decision_action(action: StringName) -> void:
	if action != &"save" or _decision_operation.is_empty() or _awaiting_world_save or _creating_world:
		return
	_awaiting_world_save = true
	_decision_dialog.get_ok_button().disabled = true
	_decision_dialog.get_cancel_button().disabled = true
	_save_world_button.disabled = true
	if world_save_requested.get_connections().is_empty():
		complete_world_save(false, "", "R_WORLD_SAVE_UNAVAILABLE")
	else:
		match _decision_operation:
			"create": world_save_requested.emit(WorldSaveReason.CREATE)
			"reset": world_save_requested.emit(WorldSaveReason.RESET)
			"quit": world_save_requested.emit(WorldSaveReason.QUIT)


func _save_world_for_decision(_reason: WorldSaveReason) -> void:
	_decision_dialog.hide()
	_begin_file_operation("world_save", Vector2.ZERO)
	if _awaiting_world_save and _file_operation.is_empty() and _preference_close_kind.is_empty():
		complete_world_save(false, "", "R_WORLD_SAVE_FAILED")


func complete_world_save(succeeded: bool, message: String = "", message_key: String = "") -> void:
	if not _awaiting_world_save or _shutting_down:
		return
	var closing: bool = _decision_operation == "quit"
	if closing and succeeded:
		_finish_preference_close("quit")
		return
	_end_world_decision(true)
	if not succeeded and (not message.is_empty() or not message_key.is_empty()):
		_show_error(message, message_key)
	if closing:
		close_cancelled.emit()


func _end_world_decision(resume_previous: bool) -> void:
	var restore_running: bool = _decision_restore_running
	_creating_world = false
	_creation_generation += 1
	_decision_dialog.dialog_hide_on_ok = true
	_decision_operation = ""
	_decision_values = PackedInt64Array()
	_decision_restore_running = false
	_awaiting_world_save = false
	_decision_dialog.hide()
	_decision_dialog.get_ok_button().disabled = false
	_decision_dialog.get_cancel_button().disabled = false
	_save_world_button.disabled = false
	if resume_previous and restore_running and not _runtime_failed:
		_accept_result(_simulation.apply_commands(PackedInt32Array([1])))
	_last_clock_usec = _clock_usec()
	get_viewport().gui_release_focus()
	if resume_previous:
		_restore_hidden_settings()


func _save_preferences_current_geometry() -> bool:
	var window: Window = get_window()
	var source_state: int = 6 if window.mode == Window.MODE_MAXIMIZED else (1 if window.mode == Window.MODE_MINIMIZED else 0)
	var rectangle: Rect2i = Rect2i(window.position, window.size)
	if window.mode in [Window.MODE_FULLSCREEN, Window.MODE_EXCLUSIVE_FULLSCREEN] and _windowed_geometry.size.x > 0:
		rectangle = _windowed_geometry
		source_state = 6 if _windowed_mode == Window.MODE_MAXIMIZED else 0
	var geometry: PackedInt32Array = PackedInt32Array([rectangle.position.x, rectangle.position.y, rectangle.size.x, rectangle.size.y, source_state])
	var result: Dictionary = _simulation.preferences_save(geometry)
	if not bool(result["ok"]):
		_show_error(str(result["error"]))
		return false
	return true


func _crisp_changed(enabled: bool) -> void:
	var result: Dictionary = _simulation.set_crisp_lines(enabled)
	if not bool(result["ok"]):
		_crisp_lines.set_pressed_no_signal(not enabled)
		_show_error(str(result["error"]))
		return
	_canvas.set_crisp_lines(enabled)
	_save_preferences_current_geometry()


func request_stop() -> void:
	if not _recovery_choice.is_empty():
		_recovery_dialog.grab_focus()
		return
	if _shutting_down or not _preference_close_kind.is_empty():
		return
	if not _file_operation.is_empty():
		if _overwrite_dialog.visible:
			_overwrite_dialog.grab_focus()
		else:
			_file_dialog.grab_focus()
		return
	if not _decision_operation.is_empty():
		_decision_dialog.grab_focus()
		return
	_finish_preference_close("stop")


func _finish_preference_close(kind: String) -> void:
	_cancel_version_check()
	if _save_preferences_current_geometry():
		_preference_close_kind = ""
		_preference_failure.hide()
		if kind == "quit":
			_end_world_decision(false)
		_close_simulation()
		return
	_preference_close_kind = kind
	if _settings_open():
		_settings_window.hide()
	_decision_dialog.hide()
	_preference_failure_detail = _error_message
	_refresh_modal_locale()
	_preference_failure.popup_centered_clamped(Vector2i(600, 230))
	_preference_failure.get_cancel_button().grab_focus()


func _retry_preference_close() -> void:
	var kind: String = _preference_close_kind
	if not kind.is_empty():
		_finish_preference_close(kind)


func _cancel_preference_close() -> void:
	var kind: String = _preference_close_kind
	_preference_close_kind = ""
	_preference_failure.hide()
	if kind == "quit":
		_cancel_world_decision()
	else:
		_restore_hidden_settings()
		close_cancelled.emit()


func _restore_hidden_settings() -> void:
	if is_instance_valid(_settings_window) and not _settings_window.visible and not bool(_settings_window.get("_closing")):
		_settings_window.popup()


func _preference_close_action(action: StringName) -> void:
	if action != &"discard" or _preference_close_kind.is_empty():
		return
	var kind: String = _preference_close_kind
	_preference_close_kind = ""
	_preference_failure.hide()
	if kind == "quit":
		_end_world_decision(false)
	_close_simulation()


func _close_simulation() -> void:
	if _shutting_down:
		return
	await shutdown()
	if not is_inside_tree():
		return
	if simulation_closed.get_connections().is_empty():
		get_tree().quit()
	else:
		simulation_closed.emit()


func shutdown(force: bool = false) -> void:
	if _shutting_down:
		if force:
			_release_simulation_world()
		else:
			while _simulation != null and is_inside_tree():
				await get_tree().process_frame
		return
	_shutting_down = true
	_recovery_choice = ""
	if is_instance_valid(_recovery_dialog):
		_recovery_dialog.hide()
	_stop_native_sampler()
	if _endurance != null:
		if _endurance.native_collecting():
			_native_configuration_changed()
		_endurance.finish(_clock_usec(), _endurance_sample(), "shutdown")
		if _endurance.native_enabled:
			_endurance.finish_native(_clock_usec())
	if is_instance_valid(_about_window):
		_about_window.close_about()
	_about_window = null
	_cancel_version_check()
	_close_network_windows()
	_close_statistics()
	if is_instance_valid(_settings_window):
		_settings_window.hide()
		_settings_window.queue_free()
	_settings_window = null
	_restore_settings_embedding()
	set_process(false)
	set_process_input(false)
	_pending_image = null
	_save_filename = ""
	_restore_running = false
	_file_operation = ""
	_decision_operation = ""
	_awaiting_world_save = false
	_creating_world = false
	_creation_generation += 1
	_automatic_source_active = false
	_state.clear()
	if not force and _simulation != null:
		var stopped: Dictionary = _simulation.network_stop()
		var stop_error: String = str(stopped["error"])
		if not bool(stopped["ok"]):
			push_error(stop_error)
		while bool(stopped["closing"]) and is_inside_tree():
			await get_tree().process_frame
			if _simulation == null or not is_inside_tree():
				break
			stopped = _simulation.network_poll_stop()
			var message: String = str(stopped["error"])
			if not bool(stopped["ok"]) and message != stop_error:
				push_error(message)
			stop_error = message
	_release_simulation_world()


func _release_simulation_world() -> void:
	if _simulation != null:
		var released: Dictionary = _simulation.release_world()
		if not bool(released["ok"]):
			push_error(str(released["error"]))
	_simulation = null


func _exit_tree() -> void:
	_stop_native_sampler()
	shutdown(true)


func _on_genetics_action(action: int) -> void:
	match action:
		0: _open_laboratory("clipboard")
		1: _open_laboratory("empty")
		2: _on_selected_genetics_action("copy")
		3: _open_laboratory("selection")
		4: _set_placement_mode("paste")
		5: _set_placement_mode("random")
		6: _set_placement_mode("import")
		7: _begin_file_operation("export", Vector2.ZERO)
		8: _begin_file_operation("image", Vector2.ZERO)


func _on_world_context_action(action: int) -> void:
	if action == 0:
		_open_laboratory("clipboard")
	elif action == 6:
		_begin_file_operation("import", _placement_position)
	else:
		_place_at(_placement_position, "paste" if action == 4 else "random")


func _on_selected_genetics_action(action: String) -> void:
	match action:
		"copy": _accept_result(_simulation.copy_selection())
		"examine": _open_laboratory("selection")
		"export": _begin_file_operation("export", Vector2.ZERO)
		"image": _begin_file_operation("image", Vector2.ZERO)


func _open_laboratory(source: String) -> void:
	var result: Dictionary = _simulation.open_laboratory(source)
	if not bool(result["ok"]):
		_show_error(str(result["error"]))
		return
	var laboratory: LaboratoryWindow = LaboratoryWindow.new()
	laboratory.hide()
	laboratory.force_native = true
	laboratory.simulation = _simulation
	laboratory.draft = result["draft"]
	laboratory.theme = theme
	laboratory.committed.connect(_accept_laboratory_commit)
	add_child(laboratory)


func _accept_laboratory_commit(result: Dictionary) -> void:
	if _accept_result(result):
		_error_label.visible = false


func _set_placement_mode(mode: String) -> void:
	if mode == "paste" and not bool(_state.get("clipboard_ready", false)):
		_show_error("", "R_PASTE_REQUIRED")
		return
	_placement_mode = mode
	_refresh_placement_hint()
	_canvas.grab_focus()


func _refresh_placement_hint() -> void:
	var key: String = "R_PLACE_RANDOM" if _placement_mode == "random" else ("R_PLACE_PASTE" if _placement_mode == "paste" else "R_PLACE_IMPORT")
	_placement_hint.text = MainLabels.message(key, _main_language)
	_placement_hint.visible = not _placement_mode.is_empty()


func _place_at(position: Vector2, mode: String) -> void:
	if mode == "import":
		_begin_file_operation("import", position)
		return
	var result: Dictionary = _simulation.paste_clipboard(position.x, position.y) if mode == "paste" else _simulation.random_organism(position.x, position.y)
	if _accept_result(result):
		if bool(result["placement_succeeded"]):
			_error_label.visible = false
		else:
			_show_error("", "R_PLACE_FAILED")


func _begin_file_operation(operation: String, position: Vector2) -> void:
	if _shutting_down or _state.is_empty() or not _file_operation.is_empty():
		return
	if not _decision_operation.is_empty() and not (_awaiting_world_save and operation == "world_save"):
		return
	_accrue_clock()
	_restore_running = not bool(_state["paused"]) and operation != "world_open"
	if not _accept_result(_simulation.apply_commands(PackedInt32Array([0]))):
		return
	_file_operation = operation
	_file_position = position
	if operation == "world_recover":
		if bool(_automatic_status.get("manual_checkpoint_pending", false)):
			_show_recovery_choice("older")
		else:
			_perform_managed_recovery(false)
		return
	var language: String = str(_simulation.settings_snapshot()["language"])
	var all_files_filter: String = FileFilterLabels.filter_entry("*", "T_ALL_FILES", language)
	_file_dialog.use_native_dialog = operation.begins_with("world_")
	if operation == "image":
		var prepared: Dictionary = _simulation.prepare_selection_image()
		if not bool(prepared["ok"]):
			_show_error(str(prepared["error"]))
			_finish_file_operation()
			return
		_pending_image = prepared["export"]
	if operation in ["world_open", "world_save", "world_save_as"]:
		if operation == "world_save" and not _world_path.is_empty():
			_save_path = _world_path
			_save_export()
			return
		_file_dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE if operation == "world_open" else FileDialog.FILE_MODE_SAVE_FILE
		_file_dialog.title = MainLabels.message("T_OPEN_WORLD" if operation == "world_open" else "T_SAVE_WORLD_AS", _main_language)
		_file_dialog.filters = PackedStringArray([FileFilterLabels.filter_entry("*.bgw", "T_BIOGENESIS_WORLD_FILES", language), all_files_filter])
		if _world_path.is_empty():
			_file_dialog.current_file = "" if operation == "world_open" else "world.bgw"
		else:
			_file_dialog.current_dir = _world_path.get_base_dir()
			_file_dialog.current_file = "" if operation == "world_open" else _world_path.get_file()
		get_viewport().gui_release_focus()
		get_node("/root/TouchKeyboard").call("hide_keyboard")
		_file_dialog.popup_centered_clamped(Vector2i(800, 560))
		return
	_file_dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE if operation == "import" else FileDialog.FILE_MODE_SAVE_FILE
	_file_dialog.title = MainLabels.message("T_SAVE_IMAGE" if operation == "image" else ("T_EXPORT_GENETIC_CODE" if operation == "export" else "T_IMPORT_GENETIC_CODE"), _main_language)
	if operation == "image":
		_file_dialog.filters = PackedStringArray([FileFilterLabels.filter_entry("*.png", "T_PNG_IMAGE_FILES", language), all_files_filter])
	elif operation == "export":
		_file_dialog.filters = PackedStringArray([FileFilterLabels.filter_entry("*.bgg", "T_BIOGENESIS_GENETIC_CODE_FILES", language), all_files_filter])
	else:
		_file_dialog.filters = PackedStringArray([FileFilterLabels.filter_entry("*.bgg", "T_BIOGENESIS_GENETIC_CODE_FILES", language), FileFilterLabels.filter_entry("*.bgg", "T_BIOGENESIS_GENETIC_CODE_FILES", language).replace("*.bgg", "*.xml"), all_files_filter])
	_file_dialog.current_file = "organism.png" if operation == "image" else ("genotype.bgg" if operation == "export" else "")
	if operation in ["image", "export"]:
		_save_filename = _file_dialog.current_file
	_file_dialog.popup_centered_clamped(Vector2i(800, 560))


func _loaded_world_ready(keep_laboratories: bool = false) -> void:
	_network_world_replaced()
	_close_statistics()
	if not keep_laboratories:
		for child: Node in get_children():
			if child is LaboratoryWindow:
				(child as LaboratoryWindow)._cancel()
	_set_placement_mode("")
	_seed_input.text = str(_state["seed"])
	_width_input.text = str(int(_state["width"]))
	_height_input.text = str(int(_state["height"]))
	_attempts_input.text = str(_state["initial_attempts"])
	_world_ready()


func _capture_save_filename(_event: InputEvent) -> void:
	# Capture the visible name before FileDialog applies its selected filter
	if _file_operation in ["image", "export"] and _file_dialog.visible:
		_save_filename = _file_dialog.get_line_edit().text.get_file()


func _on_genotype_file_selected(path: String) -> void:
	_file_dialog.hide()
	if _file_operation == "world_open":
		_close_statistics()
		var result: Dictionary = _simulation.open_manual_world_file(path)
		if _accept_result(result):
			_world_path = path
			_loaded_world_ready(true)
			_refresh_automatic_context()
		_finish_file_operation()
	elif _file_operation == "import":
		var result: Dictionary = _simulation.import_organism(path, _file_position.x, _file_position.y)
		if _accept_result(result) and not bool(result["placement_succeeded"]):
			_show_error("", "R_IMPORT_PLACE_FAILED")
		_finish_file_operation()
	elif _file_operation in ["export", "image", "world_save", "world_save_as"]:
		if _file_operation == "export":
			path = path.get_base_dir().path_join(_save_filename)
		var extension: String = ".png" if _file_operation == "image" else (".bgw" if _file_operation.begins_with("world_") else ".bgg")
		_save_path = path.get_base_dir().path_join(_save_filename) if _file_operation == "image" else (path + extension if path.get_file().get_extension().is_empty() else path)
		var result: Dictionary = _simulation.world_destination(_save_path) if _file_operation.begins_with("world_") else _simulation.genotype_destination(_save_path)
		if not bool(result["ok"]):
			var message: String = str(result["error"])
			_show_error(message)
			_finish_file_operation(false, message)
		elif bool(result["exists"]):
			_refresh_modal_locale()
			_overwrite_dialog.popup_centered_clamped(Vector2i(520, 180))
		else:
			_save_export()


func _show_recovery_choice(choice: String) -> void:
	if _shutting_down or not _recovery_choice.is_empty():
		return
	_recovery_choice = choice
	_refresh_modal_locale()
	_recovery_dialog.popup_centered_clamped(Vector2i(620, 260))
	_recovery_dialog.get_cancel_button().grab_focus()


func _confirm_recovery_choice() -> void:
	var choice: String = _recovery_choice
	if choice.is_empty():
		return
	_recovery_choice = ""
	_recovery_detail = ""
	_recovery_dialog.hide()
	if choice == "file_only":
		_save_export(true)
	else:
		_perform_managed_recovery(true)


func _cancel_recovery_choice() -> void:
	var choice: String = _recovery_choice
	_recovery_choice = ""
	_recovery_detail = ""
	_recovery_dialog.hide()
	if not _file_operation.is_empty():
		_finish_file_operation(false, MainLabels.message("T_CANCEL", _main_language))
	if choice == "older":
		_canvas.grab_focus()


func _recovery_custom_action(action: StringName) -> void:
	if action != "open" or _recovery_choice != "older":
		return
	_cancel_recovery_choice()
	_begin_file_operation("world_open", Vector2.ZERO)


func _perform_managed_recovery(acknowledged: bool) -> void:
	if _shutting_down:
		return
	var recovered: Dictionary = _simulation.recover_acknowledged_world() if acknowledged else _simulation.recover_latest_world()
	if _accept_result(recovered):
		_restore_running = false
		_world_path = ""
		_loaded_world_ready()
		_refresh_automatic_context()
	_update_automatic_status(_simulation.automatic_backup_status())
	_finish_file_operation()


func _save_export(file_only_acknowledged: bool = false) -> void:
	if not _recovery_choice.is_empty():
		return
	if _file_operation.is_empty() or _save_path.is_empty():
		return
	var world_save: bool = _file_operation in ["world_save", "world_save_as"]
	var result: Dictionary
	if world_save:
		result = _simulation.save_manual_world_file(_save_path, file_only_acknowledged)
		_update_automatic_status(_simulation.automatic_backup_status())
		if bool(result.get("file_only_acknowledgment_required", false)):
			_recovery_detail = str(result["error"])
			_show_recovery_choice("file_only")
			return
	elif _file_operation == "image":
		result = _pending_image.save_to_file(_save_path)
	else:
		result = _simulation.export_selection(_save_path)
	var succeeded: bool = bool(result["ok"])
	var message: String = str(result["error"])
	if not succeeded:
		_show_error(message)
	else:
		if world_save:
			_world_path = _save_path
			_refresh_automatic_context()
		_error_label.visible = false
		if world_save and bool(result.get("file_only_waived", false)):
			_show_error("", "R_FILE_ONLY_WARNING")
	_finish_file_operation(succeeded, message, bool(result.get("replacement_committed", false)))


func _finish_file_operation(succeeded: bool = false, message: String = "", committed: bool = false) -> void:
	if _file_operation.is_empty():
		return
	var decision_save: bool = _awaiting_world_save and _file_operation == "world_save"
	_file_operation = ""
	_save_path = ""
	_pending_image = null
	_save_filename = ""
	_file_dialog.hide()
	_overwrite_dialog.hide()
	if _automatic_source_active:
		_update_automatic_status(_simulation.complete_automatic_backup(succeeded, message, committed, _world_path))
		_automatic_source_active = false
	if decision_save:
		_restore_running = false
		complete_world_save(succeeded, message)
		return
	if _restore_running and not _runtime_failed:
		_accept_result(_simulation.apply_commands(PackedInt32Array([1])))
	_restore_running = false
	_last_clock_usec = _clock_usec()


func _network_open() -> bool:
	return is_instance_valid(_network_window) and _network_window.visible


func _open_network(mode: int) -> void:
	if _shutting_down or _simulation == null:
		return
	if _network_open():
		_network_window.grab_focus()
		return
	if _settings_open() or is_instance_valid(_about_window) or is_instance_valid(_version_window) or not _file_operation.is_empty() or not _decision_operation.is_empty() or not _preference_close_kind.is_empty():
		_show_error(NetworkLabels.message("R_BUSY", _main_language))
		return
	_network_result(_simulation.network_snapshot())
	var window: NetworkWindow = NetworkWindow.new()
	window.hide()
	window.name = "NetworkWindow"
	window.force_native = true
	window.simulation = _simulation
	window.language = _main_language
	window.initial_mode = mode
	window.initial_snapshot = _network_packet
	window.changed.connect(_network_result)
	var instance_id: int = window.get_instance_id()
	window.tree_exited.connect(_network_removed.bind(instance_id))
	if mode == 0:
		_network_window = window
	else:
		_network_managers[instance_id] = weakref(window)
	add_child(window)


func _network_removed(instance_id: int) -> void:
	if is_instance_valid(_network_window) and _network_window.get_instance_id() == instance_id:
		_network_window = null
	_network_managers.erase(instance_id)
	if not _shutting_down and is_inside_tree() and not _network_open() and is_instance_valid(_canvas) and get_window().has_focus():
		_canvas.grab_focus()


func _close_network_window() -> void:
	if is_instance_valid(_network_window):
		_network_window.close_window()


func _close_network_windows() -> void:
	_close_network_window()
	for reference: WeakRef in _network_managers.values():
		var manager: NetworkWindow = reference.get_ref() as NetworkWindow
		if manager != null:
			manager.close_window()


func _network_world_replaced() -> void:
	_close_network_windows()
	_canvas.set_network_presentation(PackedFloat64Array())
	_pump_network()


func _pump_network() -> void:
	if _simulation != null:
		_network_result(_simulation.network_pump())


func _network_result(packet: Dictionary) -> void:
	var previous_error_feedback: Variant = _network_packet.get("error_feedback")
	if not _network_packet.is_empty() and int(packet["generation"]) != int(_network_packet["generation"]):
		_close_network_windows()
	_network_packet = packet
	if packet.has("world_snapshot"):
		_accept_result(packet["world_snapshot"])
	if bool(packet.get("presentation_changed", false)):
		if not _canvas.set_network_presentation(packet["presentation"]):
			_show_error(NetworkLabels.message("R_PRESENTATION_ERROR", _main_language))
	if is_instance_valid(_network_window):
		_network_window.apply_snapshot(packet)
	for reference: WeakRef in _network_managers.values():
		var manager: NetworkWindow = reference.get_ref() as NetworkWindow
		if manager != null:
			manager.apply_snapshot(packet)
	var message: String = str(packet["error"])
	if not bool(packet["ok"]) and (message != _network_error or packet.get("error_feedback") != previous_error_feedback):
		_show_error(message, "", packet)
	_network_error = message
	_refresh_network_status()


func _refresh_network_status() -> void:
	if _network_status_label == null or _network_packet.is_empty():
		return
	var peers: Array = _network_packet["connections"]
	var text: String = NetworkLabels.feedback(_network_packet, "notice", _main_language)
	if text.is_empty() and bool(_network_packet["listening"]):
		text = NetworkLabels.message("R_LISTENING", _main_language) % int(_network_packet["bound_port"])
	elif text.is_empty() and not peers.is_empty():
		text = NetworkLabels.message("R_CONNECTIONS", _main_language) % [peers.size(), maxi(0, int(_network_packet["settings"]["max_connections"]))]
	_network_status_label.text = text
	_network_status_label.visible = not text.is_empty()
	_network_status_label.tooltip_text = NetworkLabels.message("R_LIMITS", _main_language) % [int(_network_packet["job_limit"]), int(_network_packet["incoming_limit"])]


func _on_network_popup_input(event: InputEvent) -> void:
	var popup: PopupMenu = _network_menu.get_popup()
	if _network_open() or not popup.visible or not event is InputEventKey:
		return
	var key: InputEventKey = event as InputEventKey
	if not key.pressed or key.echo or key.ctrl_pressed or key.meta_pressed or key.shift_pressed:
		return
	if key.alt_pressed and _route_menu_heading_key(key, popup):
		return
	for entry: Array in [["T_CONFIGURE_NETWORK", 0], ["T_MANAGE_CONNECTIONS", 1]]:
		if key.keycode == NetworkLabels.mnemonic(str(entry[0]), _main_language):
			popup.set_input_as_handled()
			popup.hide()
			_open_network(int(entry[1]))
			return


func _start_endurance() -> void:
	var parsed: Dictionary = EnduranceMetrics.options(OS.get_cmdline_user_args())
	if not str(parsed["error"]).is_empty():
		printerr("BGR_ENDURANCE_FAILURE " + str(parsed["error"]))
		return
	if not bool(parsed["enabled"]):
		return
	_endurance = EnduranceMetrics.new()
	_endurance.start(str(parsed["directory"]), _clock_usec(), _endurance_sample())
	_canvas.measure_submission = _endurance.active
	if bool(parsed["native"]) and _endurance.active:
		if not _endurance.start_native(str(parsed["directory"]), _clock_usec()):
			return
		var loop: NativeFrameLoop = get_tree() as NativeFrameLoop
		if loop == null or not loop.register_native_sampler(_native_phase_sample):
			_endurance.native_interruption("unsupported_or_occupied_main_loop", _clock_usec())
			return
		_native_configuration_changed()


func _observe_endurance() -> void:
	var now: int = _clock_usec()
	_endurance.observe_dispatch(now, bool(_state.get("paused", true)), _runtime_failed)
	if _endurance.due(now):
		_endurance.record(now, _endurance_sample())


func _endurance_sample() -> Dictionary:
	var queued: int = 0
	var candidates: int = 0
	var outgoing: int = 0
	var peers: Array = _network_packet.get("connections", [])
	for peer: Dictionary in peers:
		queued += int(peer["queued_incoming"])
		candidates += 1 if bool(peer["incoming_candidate"]) else 0
		outgoing += int(peer["pending_outgoing"])
	return {
		"state_available": not _state.is_empty(),
		"seed": int(_state.get("seed", 0)), "width": int(_state.get("width", 0)),
		"height": int(_state.get("height", 0)), "initial_attempts": int(_state.get("initial_attempts", 0)),
		"tick": int(_state.get("tick", 0)), "world_time": int(_state.get("world_time", 0)),
		"population": int(_state.get("population", 0)), "paused": bool(_state.get("paused", true)),
		"runtime_failed": _runtime_failed, "pending_ticks": _pending_ticks,
		"configured_delay_ms": _stored_delay_ms, "effective_delay_ms": _pacing.current_delay_ms,
		"continuous_display": _continuous_display, "overload_paused": _overload_paused,
		"crisp_lines": _crisp_lines.button_pressed,
		"history_counts": null, "history_counts_status": "unavailable_in_cached_state",
		"presentation": {"automatic": _cinema_allowed(), "last_camera_kind": _cinema_kind,
			"draw_cpu_submission": _canvas.submission_metrics()},
		"network": {
			"pending_command_jobs": int(_network_packet.get("pending_command_jobs", -1)),
			"connections": peers.size(), "queued_incoming": queued,
			"incoming_candidates": candidates, "pending_outgoing": outgoing,
			"closing": bool(_network_packet.get("closing", false)),
			"job_limit": int(_network_packet.get("job_limit", -1)),
			"incoming_limit": int(_network_packet.get("incoming_limit", -1))
		}
	}


func _native_phase_sample() -> void:
	if _endurance != null and _endurance.native_active:
		_endurance.native_phase(Engine.get_process_frames(), _clock_usec(),
			int(_state.get("tick", 0)), bool(_state.get("paused", true)), _runtime_failed)


func _stop_native_sampler() -> void:
	if _endurance == null or not _endurance.native_enabled:
		return
	var loop: NativeFrameLoop = get_tree() as NativeFrameLoop
	if loop != null:
		loop.clear_native_sampler(_native_phase_sample)


func _native_configuration_changed() -> void:
	if _endurance == null or not _endurance.native_collecting():
		return
	if not is_inside_tree() or not is_instance_valid(_canvas) or not _canvas.is_inside_tree() or get_window() == null:
		_endurance.native_interruption("configuration_unavailable", _clock_usec())
		return
	var window: Window = get_window()
	var canvas_rect: Rect2 = _canvas.get_global_rect()
	var offset: Vector2 = _canvas.native_view_offset()
	var fit: bool = _canvas.native_fit_enabled()
	var seed: int = int(_state.get("seed", 0))
	var width: int = int(_state.get("width", 0))
	var height: int = int(_state.get("height", 0))
	var attempts: int = int(_state.get("initial_attempts", 0))
	if not _native_configuration_cache.is_empty() and seed == _native_configuration_cache["seed"] and width == _native_configuration_cache["width"] and height == _native_configuration_cache["height"] and attempts == _native_configuration_cache["attempts"] and window.size == _native_configuration_cache["window_size"] and window.position == _native_configuration_cache["window_position"] and canvas_rect == _native_configuration_cache["canvas_rect"] and offset == _native_configuration_cache["offset"] and fit == _native_configuration_cache["fit"] and _view_zoom == _native_configuration_cache["zoom"] and _crisp_lines.button_pressed == _native_configuration_cache["crisp"] and _decision_operation == _native_configuration_cache["decision"]:
		return
	_native_configuration_cache = {"seed": seed, "width": width, "height": height,
		"attempts": attempts, "window_size": window.size, "window_position": window.position,
		"canvas_rect": canvas_rect, "offset": offset, "fit": fit, "zoom": _view_zoom,
		"crisp": _crisp_lines.button_pressed, "decision": _decision_operation}
	var controls: Dictionary = {}
	for name: String in ["SeedInput", "WidthInput", "HeightInput", "AttemptsInput", "CreateWorldButton", "PauseButton", "QuitButton", "FitButton", "CrispLines", "WorldCanvas"]:
		var control: Control = find_child(name, true, false) as Control
		if control != null and control.is_visible_in_tree():
			controls[name] = _native_control_rect(control)
	if _decision_dialog.visible:
		controls["ConfirmWorldDecision"] = _native_control_rect(_decision_dialog.get_ok_button())
		controls["CancelWorldDecision"] = _native_control_rect(_decision_dialog.get_cancel_button())
	var thread_setting: String = "rendering/driver/threads/thread_model"
	var thread_setting_present: bool = ProjectSettings.has_setting(thread_setting)
	_endurance.native_configuration({"seed": seed, "width": width, "height": height,
		"initial_attempts": attempts, "window_id": window.get_window_id(), "window_title": window.title,
		"window_size": [window.size.x, window.size.y], "window_position": [window.position.x, window.position.y],
		"canvas_rect": [canvas_rect.position.x, canvas_rect.position.y, canvas_rect.size.x, canvas_rect.size.y],
		"offset": [offset.x, offset.y], "zoom": _view_zoom, "fit_enabled": fit,
		"crisp_lines": _crisp_lines.button_pressed, "decision": _decision_operation,
		"tick": int(_state.get("tick", 0)), "paused": bool(_state.get("paused", true)),
		"state_available": not _state.is_empty(), "controls": controls,
		"screen_refresh_hz": null, "screen_refresh_status": "UNKNOWN",
		"max_fps": Engine.max_fps,
		"thread_setting_present": thread_setting_present,
		"thread_setting": int(ProjectSettings.get_setting_with_override(thread_setting)) if thread_setting_present else -1,
		"thread_setting_status": "PROJECT_SETTING_PRESENT" if thread_setting_present else "UNKNOWN",
		"display_server": DisplayServer.get_name(),
		"rendering_method": RenderingServer.get_current_rendering_method(),
		"gpu_name": RenderingServer.get_video_adapter_name(),
		"rendering_driver": null, "rendering_driver_status": "UNKNOWN_EXTERNAL_RECEIPT_REQUIRED",
		"vsync_mode": DisplayServer.window_get_vsync_mode(window.get_window_id())},
		_clock_usec(), bool(_state.get("paused", true)))


func _native_control_rect(control: Control) -> Array[float]:
	var window: Window = control.get_window()
	while window.is_embedded():
		var parent: Node = window.get_parent()
		if parent == null:
			return []
		window = parent.get_window()
	var transform: Transform2D = control.get_viewport().get_screen_transform() * control.get_global_transform_with_canvas()
	transform.origin += Vector2(DisplayServer.window_get_position(window.get_window_id()))
	var rect: Rect2 = transform * Rect2(Vector2.ZERO, control.size)
	var origin: Vector2i = DisplayServer.window_get_position(get_window().get_window_id())
	return [rect.position.x - origin.x, rect.position.y - origin.y, rect.size.x, rect.size.y]
