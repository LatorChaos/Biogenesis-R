extends VBoxContainer

signal action_requested(action: String)
signal genetics_requested(action: String)
signal preview_copy_requested
signal tracking_requested
signal abort_tracking_requested

const LIVING_ACTIONS: Array[String] = ["feed", "weaken", "reproduce", "rejuvenate", "kill"]
const DEAD_ACTIONS: Array[String] = ["revive", "disperse"]
const GenomePreview = preload("res://ui/genome_preview.gd")
const Labels = preload("res://ui/main_labels.gd")
const ACTION_KEYS: Dictionary[String, String] = {
    "feed": "T_FEED", "weaken": "T_WEAKEN", "reproduce": "T_FORCE_REPRODUCTION",
    "rejuvenate": "T_REJUVENATE", "kill": "T_KILL", "revive": "T_REVIVE", "disperse": "T_DISPERSE"
}

var _details: VBoxContainer
var _preview: GenomePreview
var _summary: Label
var _lineage: Label
var _actions: HFlowContainer
var _buttons: Dictionary[String, Button] = {}
var _track_button: Button
var _abort_button: Button
var _popup: PopupMenu
var _popup_actions: Dictionary[int, String] = {}
var _selected: Dictionary = {}
var _selected_is_tracked: bool = false
var _has_tracking: bool = false
var _language: String = "en"
var _preview_error: String = ""


func _ready() -> void:
	name = "OrganismControls"
	add_theme_constant_override("separation", 4)
	var row: HBoxContainer = HBoxContainer.new()
	add_child(row)
	_preview = GenomePreview.new()
	_preview.name = "SelectedGenomePreview"
	_preview.custom_minimum_size = Vector2(100, 100)
	_preview.copy_enabled = true
	_preview.copy_requested.connect(func() -> void: preview_copy_requested.emit())
	row.add_child(_preview)
	_details = VBoxContainer.new()
	_details.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(_details)
	_summary = _information_label("SelectedOrganismSummary")
	_lineage = _information_label("SelectedOrganismLineage")
	_actions = HFlowContainer.new()
	_actions.add_theme_constant_override("h_separation", 6)
	_actions.add_theme_constant_override("v_separation", 4)
	_details.add_child(_actions)
	for action: String in LIVING_ACTIONS + DEAD_ACTIONS:
		var button: Button = Button.new()
		button.name = action.capitalize() + "OrganismButton"
		button.pressed.connect(_request_action.bind(action))
		_actions.add_child(button)
		_buttons[action] = button
	_track_button = Button.new()
	_track_button.name = "TrackOrganismButton"
	_track_button.pressed.connect(_request_tracking)
	_actions.add_child(_track_button)
	_abort_button = Button.new()
	_abort_button.name = "AbortOrganismTrackingButton"
	_abort_button.pressed.connect(_request_abort_tracking)
	_actions.add_child(_abort_button)
	_popup = PopupMenu.new()
	_popup.name = "OrganismPopup"
	_popup.id_pressed.connect(_on_popup_id_pressed)
	add_child(_popup)
	set_language(_language)
	visible = false


func _information_label(node_name: String) -> Label:
	var label: Label = Label.new()
	label.name = node_name
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.max_lines_visible = 3
	label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.add_theme_font_size_override("font_size", 13)
	_details.add_child(label)
	return label


func set_genome_preview(preview: Dictionary, error: String) -> bool:
	if not _preview.set_raster(preview):
		return false
	_preview_error = error
	_preview.tooltip_text = error if not error.is_empty() else Labels.message("T_COPY_GENETIC_CODE", _language)
	return true


func set_selection(data: Dictionary, selected_is_tracked: bool, has_tracking: bool) -> void:
	_selected = data
	_selected_is_tracked = selected_is_tracked
	_has_tracking = has_tracking
	visible = not data.is_empty()
	if data.is_empty():
		_popup.hide()
		return
	var alive: bool = bool(data["alive"])
	var genotype_available: bool = bool(data["genotype_available"])
	var display: Dictionary = data["display"]
	var unavailable: String = Labels.message("R_GENOTYPE_UNAVAILABLE", _language)
	var generation: String = str(display["generation"]) if genotype_available else unavailable
	var mutation: String = str(display["mutation_rate"]) if genotype_available else unavailable
	var clade: String = str(display["clade"]) if genotype_available else unavailable
	_summary.text = "   ".join(PackedStringArray([
		_caption("T_ID", display["id"]), _caption("T_GENERATION", generation),
		_caption("T_AGE", display["age"]), _caption("T_LIFE_EXPECTANCY", display["maximum_age"]),
		_caption("T_ENERGY", display["energy"]), _caption("T_MASS", display["mass"])
	]))
	_summary.tooltip_text = _summary.text
	_summary.accessibility_name = _summary.text
	_lineage.text = "   ".join(PackedStringArray([
		_caption("T_CHILDREN", display["children"]), _caption("T_VICTIMS", display["kills"]),
		_caption("T_INFECTED", display["infections"]), _caption("T_REPRODUCTION", display["reproduce_energy"]),
		_caption("T_MUTATION_PERCENTAGE", mutation), _caption("T_CLADE_ID", clade)
	]))
	_lineage.tooltip_text = _lineage.text
	_lineage.accessibility_name = _lineage.text
	for action: String in _buttons:
		_buttons[action].visible = action in (LIVING_ACTIONS if alive else DEAD_ACTIONS)
	_track_button.visible = alive
	_track_button.text = Labels.message("T_ABORT_TRACKING" if selected_is_tracked else "T_TRACK", _language)
	_track_button.accessibility_name = _track_button.text
	_abort_button.visible = alive and has_tracking and not selected_is_tracked


func _caption(key: String, value: String) -> String:
	return Labels.message(key, _language).strip_edges() + " " + value


func set_language(language: String) -> void:
	_language = language
	for action: String in _buttons:
		var caption: String = Labels.message(ACTION_KEYS[action], _language)
		_buttons[action].text = caption
		_buttons[action].accessibility_name = caption
	_abort_button.text = Labels.message("T_ABORT_TRACKING", _language)
	_abort_button.accessibility_name = _abort_button.text
	_preview.set_copy_caption(Labels.message("T_COPY_GENETIC_CODE", _language))
	_preview.accessibility_name = Labels.message("T_COPY_GENETIC_CODE", _language)
	_preview.tooltip_text = _preview_error if not _preview_error.is_empty() else Labels.message("T_COPY_GENETIC_CODE", _language)
	_popup.hide()
	set_selection(_selected, _selected_is_tracked, _has_tracking)


func show_context(screen_position: Vector2) -> void:
	if _selected.is_empty():
		return
	_popup.clear()
	_popup_actions.clear()
	var alive: bool = bool(_selected["alive"])
	for action: String in LIVING_ACTIONS if alive else DEAD_ACTIONS:
		_add_popup_action(action, Labels.message(ACTION_KEYS[action], _language))
	_popup.add_separator()
	_add_popup_action("examine", Labels.message("T_EXAMINE_GENES", _language))
	if alive:
		_add_popup_action("copy", Labels.message("T_COPY_GENETIC_CODE", _language))
		_add_popup_action("export", Labels.message("T_EXPORT_GENETIC_CODE", _language))
		_add_popup_action("image", Labels.message("T_SAVE_IMAGE", _language))
	if alive:
		_popup.add_separator()
		_add_popup_action("track", Labels.message("T_ABORT_TRACKING" if _selected_is_tracked else "T_TRACK", _language))
	if alive and _has_tracking and not _selected_is_tracked:
		_add_popup_action("abort_tracking", Labels.message("T_ABORT_TRACKING", _language))
	_popup.position = Vector2i(screen_position)
	_popup.popup()


func has_open_popup() -> bool:
	return _popup.visible or _preview.has_open_popup()


func _add_popup_action(action: String, label: String) -> void:
	var id: int = _popup.item_count
	_popup_actions[id] = action
	_popup.add_item(label, id)
	_popup.set_item_disabled(id, not bool(_selected["genotype_available"]) and action == "export")


func _on_popup_id_pressed(id: int) -> void:
	assert(_popup_actions.has(id), "Unknown organism menu item")
	var action: String = _popup_actions[id]
	if action == "track":
		_request_tracking()
	elif action == "abort_tracking":
		_request_abort_tracking()
	elif action in ["examine", "copy", "export", "image"]:
		genetics_requested.emit(action)
	else:
		_request_action(action)


func _request_action(action: String) -> void:
	action_requested.emit(action)


func _request_tracking() -> void:
	tracking_requested.emit()


func _request_abort_tracking() -> void:
	abort_tracking_requested.emit()
