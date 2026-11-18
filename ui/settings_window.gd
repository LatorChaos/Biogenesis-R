extends Window

signal applied(result: Dictionary)
signal finished

const MainLabels = preload("res://ui/main_labels.gd")
const DisplayPreferences = preload("res://ui/display_preferences.gd")
const SettingsFields = preload("res://ui/settings_fields.gd")
const LANGUAGE_CODES: PackedStringArray = ["en", "es", "ca", "de", "pt", "ru"]

var display_preferences: DisplayPreferences
var _cinema_motion: SpinBox
var _cinema_hold: SpinBox
var _cinema_motion_label: Label
var _cinema_hold_label: Label
var _cinema_note: Label

var simulation: BgrSimulation
var draft: BgrSettingsDraft
var locale_callback: Callable
var _data: Dictionary = {}
var _editors: Array[LineEdit] = []
var _labels: Array[Label] = []
var _tabs: TabContainer
var _locale: OptionButton
var _backup: CheckBox
var _backup_mode: OptionButton
var _hardware: OptionButton
var _fbo: CheckBox
var _language_label: Label
var _locale_notice: Label
var _graphics_label: Label
var _error: Label
var _ok: Button
var _cancel: Button
var _defaults: Button
var _language: String = "en"
var _closing: bool = false
var _disclosure: Label
var _graphics_note: Label


func _ready() -> void:
	transient = true
	exclusive = true
	min_size = Vector2i(640, 460)
	size = Vector2i(760, 580)
	close_requested.connect(_cancel_draft)
	window_input.connect(_on_window_input)
	_data = SettingsFields.DATA
	assert((_data["fields"] as Array).size() == 132, "The source settings form requires 132 editors")
	assert((_data["tabs"] as Array).size() == 7, "The source settings form requires seven tabs")
	_editors.resize(132)
	_labels.resize(132)
	if display_preferences == null:
		display_preferences = DisplayPreferences.new()
		var loaded: Dictionary = display_preferences.load_settings()
		if not bool(loaded["ok"]):
			push_error(str(loaded["error"]))
	_build()
	_refresh_cinema()
	var state: Dictionary = simulation.settings_snapshot()
	_language = str(state["language"])
	_refresh(draft.snapshot())
	apply_locale(_language)
	popup_centered_clamped(size)


func _build() -> void:
	var margin: MarginContainer = MarginContainer.new()
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for side: String in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 12)
	add_child(margin)
	var column: VBoxContainer = VBoxContainer.new()
	column.add_theme_constant_override("separation", 10)
	margin.add_child(column)
	_disclosure = Label.new()
	_disclosure.text = MainLabels.message("R_SETTINGS_DISCLOSURE", _language)
	_disclosure.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	column.add_child(_disclosure)
	_tabs = TabContainer.new()
	_tabs.name = "SettingsTabs"
	_tabs.size_flags_vertical = Control.SIZE_EXPAND_FILL
	column.add_child(_tabs)
	var boxes: Dictionary = {}
	for tab: Dictionary in _data["tabs"]:
		var scroll: ScrollContainer = ScrollContainer.new()
		scroll.name = str(tab["id"])
		scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
		_tabs.add_child(scroll)
		var box: VBoxContainer = VBoxContainer.new()
		box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		box.add_theme_constant_override("separation", 8)
		scroll.add_child(box)
		boxes[str(tab["id"])] = box
	var general: VBoxContainer = boxes["General"] as VBoxContainer
	var language_row: HBoxContainer = HBoxContainer.new()
	general.add_child(language_row)
	_language_label = Label.new()
	_language_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	language_row.add_child(_language_label)
	_locale = OptionButton.new()
	_locale.name = "SettingsLanguage"
	for index: int in range(6):
		_locale.add_item(LANGUAGE_CODES[index], index)
	language_row.add_child(_locale)
	_locale_notice = Label.new()
	_locale_notice.name = "UnsupportedPreferenceLocale"
	_locale_notice.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	general.add_child(_locale_notice)
	for field: Dictionary in _data["fields"]:
		var key: String = str(field["key"])
		var index: int = int(field["fixed_order"])
		var box: VBoxContainer = boxes[str(field["tab"])] as VBoxContainer
		if key == "BACKUP_DELAY":
			_backup = CheckBox.new()
			_backup.name = "SettingsAutomaticBackup"
			_backup.tooltip_text = MainLabels.message("R_SETTINGS_BACKUP_HELP", _language)
			_backup.toggled.connect(_choices_changed)
			box.add_child(_backup)
			_backup_mode = OptionButton.new()
			_backup_mode.name = "SettingsAutomaticMode"
			_backup_mode.add_item(MainLabels.message("R_SETTINGS_ADJACENT", _language), 0)
			_backup_mode.add_item(MainLabels.message("R_SETTINGS_MANAGED", _language), 1)
			_backup_mode.tooltip_text = MainLabels.message("R_SETTINGS_BACKUP_MODE_HELP", _language)
			box.add_child(_backup_mode)
		for section: Dictionary in _data["sections"]:
			if str(section["before_key"]) == key:
				var heading: Label = Label.new()
				heading.name = str(section["label_key"])
				heading.add_theme_font_size_override("font_size", 17)
				heading.text = str(section["label_key"])
				box.add_child(heading)
		var row: HBoxContainer = HBoxContainer.new()
		box.add_child(row)
		var label: Label = Label.new()
		label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		row.add_child(label)
		_labels[index] = label
		var input: LineEdit = LineEdit.new()
		input.name = "Setting_" + key
		input.custom_minimum_size.x = 180
		input.tooltip_text = key
		input.accessibility_name = key
		row.add_child(input)
		_editors[index] = input
	_graphics_label = Label.new()
	general.add_child(_graphics_label)
	_graphics_note = Label.new()
	_graphics_note.text = MainLabels.message("R_SETTINGS_GRAPHICS_NOTE", _language)
	_graphics_note.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	general.add_child(_graphics_note)
	_hardware = OptionButton.new()
	_hardware.name = "SettingsLegacyGraphics"
	_hardware.add_item("None", 0)
	_hardware.add_item("OpenGL", 1)
	_hardware.item_selected.connect(_choices_changed)
	general.add_child(_hardware)
	_fbo = CheckBox.new()
	_fbo.name = "SettingsDisableFramebuffer"
	general.add_child(_fbo)
	_build_cinema()
	_error = Label.new()
	_error.name = "SettingsError"
	_error.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_error.add_theme_color_override("font_color", Color("edbd91"))
	_error.hide()
	column.add_child(_error)
	var buttons: HBoxContainer = HBoxContainer.new()
	buttons.alignment = BoxContainer.ALIGNMENT_END
	column.add_child(buttons)
	_ok = Button.new()
	_ok.name = "SettingsOK"
	_ok.pressed.connect(_submit)
	buttons.add_child(_ok)
	_cancel = Button.new()
	_cancel.name = "SettingsCancel"
	_cancel.pressed.connect(_cancel_draft)
	buttons.add_child(_cancel)
	_defaults = Button.new()
	_defaults.name = "SettingsDefaults"
	_defaults.pressed.connect(_use_defaults)
	buttons.add_child(_defaults)


func _build_cinema() -> void:
	var box: VBoxContainer = VBoxContainer.new()
	box.name = "Cinema"
	box.add_theme_constant_override("separation", 12)
	_tabs.add_child(box)
	_cinema_note = Label.new()
	_cinema_note.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(_cinema_note)
	_cinema_motion_label = Label.new()
	box.add_child(_cinema_motion_label)
	_cinema_motion = SpinBox.new()
	_cinema_motion.name = "CinemaMotionPercent"
	_cinema_motion.min_value = 1.0
	_cinema_motion.max_value = 100.0
	_cinema_motion.step = 0.5
	_cinema_motion.suffix = "%"
	box.add_child(_cinema_motion)
	_cinema_hold_label = Label.new()
	box.add_child(_cinema_hold_label)
	_cinema_hold = SpinBox.new()
	_cinema_hold.name = "CinemaSettledHoldSeconds"
	_cinema_hold.min_value = 5.0
	_cinema_hold.max_value = 3600.0
	_cinema_hold.step = 1.0
	_cinema_hold.suffix = "s"
	box.add_child(_cinema_hold)


func _refresh_cinema() -> void:
	_cinema_motion.value = float(display_preferences.values["camera_motion_percent"])
	_cinema_hold.value = float(display_preferences.values["camera_settled_hold_seconds"])


func _message(key: String) -> String:
	var language: String = _language if (_data["messages"] as Dictionary).has(_language) else "en"
	return str((_data["messages"][language] as Dictionary)[key])


func apply_locale(language: String) -> void:
	_language = language
	if _data.is_empty():
		return
	title = _message("T_PARAMETERS_CONFIGURATION")
	for index: int in range(7):
		_tabs.set_tab_title(index, _message(str(_data["tabs"][index]["label_key"])))
	_tabs.set_tab_title(7, MainLabels.message("R_CINEMA_TAB", language))
	_cinema_motion_label.text = MainLabels.message("R_CINEMA_MOTION", language)
	_cinema_hold_label.text = MainLabels.message("R_CINEMA_HOLD", language)
	_cinema_note.text = MainLabels.message("R_CINEMA_NOTE", language)
	_cinema_motion.get_line_edit().accessibility_name = _cinema_motion_label.text
	_cinema_hold.get_line_edit().accessibility_name = _cinema_hold_label.text
	for field: Dictionary in _data["fields"]:
		var parts: PackedStringArray = PackedStringArray()
		for key: String in field["label_keys"]:
			parts.append(_message(key))
		var caption: String = " - ".join(parts)
		if field.has("unit_key"):
			caption += " (" + _message(str(field["unit_key"])) + ")"
		_labels[int(field["fixed_order"])].text = caption
		_editors[int(field["fixed_order"])].accessibility_name = caption
	_language_label.text = _message("T_LANGUAGE")
	_locale_notice.visible = not LANGUAGE_CODES.has(language)
	_locale_notice.text = MainLabels.message("R_SETTINGS_UNSUPPORTED_LOCALE", _language) % [language.left(96), int(simulation.settings_snapshot()["locale"])]
	for index: int in range(6):
		_locale.set_item_text(index, _message(str(_data["languages"][index])))
	for section: Dictionary in _data["sections"]:
		var heading: Label = find_child(str(section["label_key"]), true, false) as Label
		heading.text = _message(str(section["label_key"]))
	_disclosure.text = MainLabels.message("R_SETTINGS_DISCLOSURE", _language)
	_backup.tooltip_text = MainLabels.message("R_SETTINGS_BACKUP_HELP", _language)
	_backup_mode.set_item_text(0, MainLabels.message("R_SETTINGS_ADJACENT", _language))
	_backup_mode.set_item_text(1, MainLabels.message("R_SETTINGS_MANAGED", _language))
	_backup_mode.tooltip_text = MainLabels.message("R_SETTINGS_BACKUP_MODE_HELP", _language)
	_graphics_note.text = MainLabels.message("R_SETTINGS_GRAPHICS_NOTE", _language)
	_backup.text = _message("T_AUTOMATIC_BACKUPS")
	_graphics_label.text = _message("T_HARDWARE_ACCELERATION")
	_hardware.set_item_text(0, _message("T_NONE"))
	_hardware.set_item_text(1, _message("T_OPENGL"))
	_fbo.text = _message("T_DISABLE_FBOBJECT")
	_ok.text = _message("T_OK")
	_cancel.text = _message("T_CANCEL")
	_defaults.text = _message("T_DEFAULT_VALUES")


func _refresh(snapshot: Dictionary) -> void:
	if not bool(snapshot["ok"]):
		_show_error(str(snapshot["error"]))
		_ok.disabled = true
		return
	var keys: PackedStringArray = snapshot["keys"]
	var texts: PackedStringArray = snapshot["texts"]
	assert(keys.size() == 132 and texts.size() == 132)
	for field: Dictionary in _data["fields"]:
		var index: int = int(field["fixed_order"])
		assert(keys[index] == str(field["key"]), "Settings field order differs from Core")
		_editors[index].text = texts[index]
	_locale.select(int(snapshot["locale"]))
	_backup.set_pressed_no_signal(bool(snapshot["automatic_backup"]))
	_backup_mode.select(int(snapshot["automatic_backup_mode"]))
	_hardware.select(int(snapshot["hardware"]))
	_fbo.set_pressed_no_signal(bool(snapshot["disable_framebuffer"]))
	_choices_changed()


func _choices_changed(_value: Variant = null) -> void:
	for field: Dictionary in _data["fields"]:
		if str(field["key"]) == "BACKUP_DELAY":
			var input: LineEdit = _editors[int(field["fixed_order"])]
			input.editable = _backup.button_pressed
			input.focus_mode = Control.FOCUS_ALL if _backup.button_pressed else Control.FOCUS_NONE
	_fbo.disabled = _hardware.selected == 0


func _capture() -> bool:
	var texts: PackedStringArray = PackedStringArray()
	for editor: LineEdit in _editors:
		texts.append(editor.text)
	var result: Dictionary = draft.replace_controls({"texts": texts, "locale": _locale.selected, "automatic_backup": _backup.button_pressed, "hardware": _hardware.selected, "disable_framebuffer": _fbo.button_pressed, "automatic_backup_mode": _backup_mode.selected})
	if not bool(result["ok"]):
		_show_error(str(result["error"]))
		return false
	return true


func _use_defaults() -> void:
	if not _capture():
		return
	var result: Dictionary = draft.defaults()
	_refresh(result)
	_cinema_motion.value = float(DisplayPreferences.DEFAULTS["camera_motion_percent"])
	_cinema_hold.value = float(DisplayPreferences.DEFAULTS["camera_settled_hold_seconds"])


func _submit() -> void:
	if _closing or not _capture():
		return
	_cinema_motion.apply()
	_cinema_hold.apply()
	var result: Dictionary = simulation.settings_apply(draft, locale_callback)
	if not bool(result["ok"]):
		_show_error(str(result["error"]))
		return
	var previous: Dictionary = display_preferences.values.duplicate()
	display_preferences.values["camera_motion_percent"] = _cinema_motion.value
	display_preferences.values["camera_settled_hold_seconds"] = _cinema_hold.value
	var saved: Dictionary = display_preferences.save_settings()
	if not bool(saved["ok"]):
		display_preferences.values = previous
		result["display_error"] = str(saved["error"])
	applied.emit(result)
	_finish()


func _cancel_draft() -> void:
	if _closing:
		return
	var result: Dictionary = draft.cancel()
	if not bool(result["ok"]):
		_show_error(str(result["error"]))
		return
	_finish()


func _finish() -> void:
	_closing = true
	finished.emit()
	queue_free()


func _show_error(message: String) -> void:
	_error.text = message
	_error.show()


func _on_window_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo and event.physical_keycode == KEY_ESCAPE:
		for child_window: Window in get_embedded_subwindows():
			if child_window.has_focus():
				return
		_cancel_draft()
		get_viewport().set_input_as_handled()


func _exit_tree() -> void:
	if draft == null:
		return
	var state: Dictionary = draft.snapshot()
	if bool(state.get("ok", false)) and bool(state.get("open", false)):
		var result: Dictionary = draft.cancel()
		if not bool(result["ok"]):
			push_error("Cannot close the owned settings draft: " + str(result["error"]))
