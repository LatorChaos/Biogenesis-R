extends Window

signal committed(result: Dictionary)

const MainLabels = preload("res://ui/main_labels.gd")
const GenomePreview = preload("res://ui/genome_preview.gd")
const FileFilterLabels = preload("res://ui/file_filter_labels.gd")
const REACTIONS: Array[String] = ["green", "bark", "red", "fire", "orange", "maroon", "pink", "cream", "silver", "spike", "lilac", "gray", "violet", "olive", "sky", "blue", "ochre", "fallow", "white", "virus", "plague", "scourge", "coral", "mint", "magenta", "default", "consumer", "plant", "ice", "lightblue", "lightbrown", "greenbrown", "broken", "brown", "sick", "friend"]
const STORAGE_REACTIONS: Array[String] = ["red", "green", "blue", "plague", "scourge", "white", "gray", "silver", "default", "consumer", "plant", "magenta", "pink", "coral", "orange", "bark", "violet", "virus", "maroon", "olive", "mint", "cream", "spike", "fallow", "lightblue", "ochre", "sky", "lilac", "fire", "lightbrown", "greenbrown", "brown", "ice", "broken", "sick", "friend"]
const COLOR_NAMES: Array[String] = ["Green", "Forest", "Spring", "Summer", "Lime", "Leaf", "C4", "Jade", "Grass", "Bark", "Purple", "Red", "Fire", "Orange", "Maroon", "Pink", "Cream", "Silver", "Spike", "Lilac", "Gray", "Violet", "Olive", "Sky", "Blue", "Ochre", "Fallow", "Spore", "White", "Plague", "Coral", "Mint", "Lavender", "Magenta", "Rose", "Cyan", "Teal", "Yellow", "Auburn", "Indigo", "Blond", "Flower", "Dark Gray", "Gold", "Dark", "Eye"]
const COLOR_VALUES: Array[int] = [0xff00ff00, 0xff008000, 0xff00ff80, 0xff80ff40, 0xffb0ff00, 0xff5cb800, 0xff60c060, 0xff00a86b, 0xff90b040, 0xff608040, 0xffa80054, 0xffff0000, 0xffff6400, 0xffffc800, 0xff800000, 0xffffafaf, 0xffd0c08c, 0xffc0c0c0, 0xffa48464, 0xffc080ff, 0xff808080, 0xff800080, 0xffb0b000, 0xff80c0ff, 0xff0000ff, 0xffcc7722, 0xff967117, 0xff0050a0, 0xffffffff, 0xffffc0ff, 0xffff648a, 0xffa0e0a0, 0xff8060b0, 0xffff00ff, 0xffff0080, 0xff00ffff, 0xff008080, 0xffffff00, 0xff803030, 0xff6f00ff, 0xffffff80, 0xff8080ff, 0xff404040, 0xffd4af37, 0xff381c0e, 0xff004040]
const NUMERIC_PROPERTIES: Array[String] = ["symmetry", "mirror", "mutation_rate", "activity", "cream", "fallow", "spore", "adapt_spore"]
const BOOLEAN_PROPERTIES: Array[String] = ["plague", "disperse_children", "generation_battle", "sibling_battle", "altruist", "familial", "social", "peaceful", "passive", "clockwise", "mimic_all", "modifies_pink", "modifies_lilac", "modifies_sky", "selfish"]

const PROPERTY_LABELS: Dictionary = {"activity": "T_ACTIVITY", "adapt_spore": "T_ADAPTSPORE", "altruist": "T_ALTRUIST", "clockwise": "T_CLOCKWISE", "cream": "T_MODIFIESCREAM", "disperse_children": "T_DISPERSE_CHILDREN", "fallow": "T_MODIFIESFALLOW", "familial": "T_FAMILIAL", "generation_battle": "T_GENERATION_BATTLE", "mimic_all": "T_MIMICALL", "mirror": "T_MIRROR", "modifies_lilac": "T_MODIFIESLILAC", "modifies_pink": "T_MODIFIESPINK", "modifies_sky": "T_MODIFIESSKY", "mutation_rate": "T_MUTATION_PERCENTAGE", "passive": "T_PASSIVE", "peaceful": "T_PEACEFUL", "plague": "T_MODIFIESPLAGUE", "selfish": "T_SELFISH", "sibling_battle": "T_SIBLING_BATTLE", "social": "T_SOCIAL", "spore": "T_MODIFIESSPORE", "symmetry": "T_SYMMETRY"}
const REACTION_LABELS: Dictionary = {"brown": "T_BRO", "greenbrown": "T_GBR", "lightblue": "T_LBL", "lightbrown": "T_LBR"}

var simulation: BgrSimulation
var draft: BgrGenomeDraft
var _state: Dictionary = {}
var _selected_gene: int = -1
var _gene_list: ItemList
var _gene_editor: VBoxContainer
var _general: GridContainer
var _preview: GenomePreview
var _estimates: Label
var _error: Label
var _import_dialog: FileDialog
var _invalid: Dictionary[String, String] = {}
var _closed: bool = false
var _language: String = "en"


func _ready() -> void:
	_language = str(simulation.settings_snapshot()["language"])
	title = "Biogenesis-R - " + _message("T_GENETIC_LABORATORY")
	name = "Laboratory"
	min_size = Vector2i(760, 620)
	size = Vector2i(1060, 780)
	transient = true
	exclusive = false
	close_requested.connect(_cancel)
	window_input.connect(_on_window_input)
	var margin: MarginContainer = MarginContainer.new()
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for side: String in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 12)
	add_child(margin)
	var column: VBoxContainer = VBoxContainer.new()
	margin.add_child(column)
	var buttons: HFlowContainer = HFlowContainer.new()
	column.add_child(buttons)
	_add_button(buttons, _message("T_COPY_TO_CLIPBOARD"), "CommitGenome", _commit)
	_add_button(buttons, _message("T_CANCEL"), "CancelLaboratory", _cancel)
	_add_button(buttons, _message("T_CLEAR"), "ClearGenes", _clear)
	_add_button(buttons, _message("T_IMPORT_GENETIC_CODE"), "ImportLaboratory", _import)
	_error = Label.new()
	_error.name = "LaboratoryError"
	_error.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_error.add_theme_color_override("font_color", Color("ffb2a9"))
	column.add_child(_error)
	var overview: HBoxContainer = HBoxContainer.new()
	column.add_child(overview)
	_preview = GenomePreview.new()
	_preview.name = "LaboratoryPreview"
	_preview.custom_minimum_size = Vector2(180, 180)
	overview.add_child(_preview)
	_preview.resized.connect(_refresh_preview)
	_estimates = Label.new()
	_estimates.name = "GenomeEstimates"
	_estimates.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_estimates.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	overview.add_child(_estimates)
	var tabs: TabContainer = TabContainer.new()
	tabs.name = "LaboratoryTabs"
	tabs.size_flags_vertical = Control.SIZE_EXPAND_FILL
	column.add_child(tabs)
	var gene_tab: HBoxContainer = HBoxContainer.new()
	gene_tab.name = "Genes"
	tabs.add_child(gene_tab)
	tabs.set_tab_title(0, _message("T_GENES"))
	var list_column: VBoxContainer = VBoxContainer.new()
	list_column.custom_minimum_size.x = 180
	gene_tab.add_child(list_column)
	_gene_list = ItemList.new()
	_gene_list.name = "GeneList"
	_gene_list.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_gene_list.item_selected.connect(_select_gene)
	list_column.add_child(_gene_list)
	_add_button(list_column, _message("T_ADD_GENE"), "AddGene", _add_gene)
	_add_button(list_column, _message("T_INSERT_GENE"), "InsertGene", _insert_gene)
	_add_button(list_column, _message("T_DELETE"), "DeleteGene", _delete_gene)
	var gene_scroll: ScrollContainer = ScrollContainer.new()
	gene_scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	gene_tab.add_child(gene_scroll)
	_gene_editor = VBoxContainer.new()
	_gene_editor.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	gene_scroll.add_child(_gene_editor)
	var general_scroll: ScrollContainer = ScrollContainer.new()
	general_scroll.name = "Genome properties"
	tabs.add_child(general_scroll)
	tabs.set_tab_title(1, _message("R_LAB_PROPERTIES"))
	_general = GridContainer.new()
	_general.columns = 2
	_general.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	general_scroll.add_child(_general)
	_import_dialog = FileDialog.new()
	FileFilterLabels.fit_filter_labels(_import_dialog)
	_import_dialog.name = "LaboratoryImportDialog"
	_import_dialog.access = FileDialog.ACCESS_FILESYSTEM
	var documents: String = OS.get_system_dir(OS.SYSTEM_DIR_DOCUMENTS)
	_import_dialog.current_dir = documents if DirAccess.dir_exists_absolute(documents) else OS.get_user_data_dir()
	_import_dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE
	_import_dialog.file_selected.connect(_import_selected)
	add_child(_import_dialog)
	_refresh(true)
	var parent_viewport: Viewport = get_parent().get_viewport()
	# The embedder applies its own client clamp after emitting size_changed
	parent_viewport.size_changed.connect(_clamp_embedded_decorations, CONNECT_DEFERRED)
	popup_centered_clamped(size)
	_clamp_embedded_decorations()


func _clamp_embedded_decorations() -> void:
	if _closed or not is_embedded() or borderless:
		return
	var decoration: Rect2 = Rect2(Vector2.ZERO, Vector2(size))
	for style_name: String in ["embedded_border", "embedded_unfocused_border"]:
		var border: StyleBoxFlat = get_theme_stylebox(style_name) as StyleBoxFlat
		if border != null:
			var painted: Rect2 = Rect2(Vector2.ZERO, Vector2(size)).grow_individual(border.get_expand_margin(SIDE_LEFT), border.get_expand_margin(SIDE_TOP), border.get_expand_margin(SIDE_RIGHT), border.get_expand_margin(SIDE_BOTTOM))
			if border.shadow_size > 0:
				var shadow: Rect2 = painted.grow(float(border.shadow_size))
				shadow.position += border.shadow_offset
				painted = painted.merge(shadow)
			decoration = decoration.merge(painted)
	var close_position: Vector2 = Vector2(float(size.x - get_theme_constant("close_h_offset")), -float(get_theme_constant("close_v_offset")))
	for icon_name: String in ["close", "close_pressed"]:
		decoration = decoration.merge(Rect2(close_position, get_theme_icon(icon_name).get_size()))
	var available: Rect2 = get_parent().get_viewport().get_visible_rect()
	var minimum_position: Vector2i = Vector2i(ceili(available.position.x - decoration.position.x), ceili(available.position.y - decoration.position.y))
	var maximum_edge: Vector2i = Vector2i(floori(available.end.x - (decoration.end.x - float(size.x))), floori(available.end.y - (decoration.end.y - float(size.y))))
	var capacity: Vector2i = maximum_edge - minimum_position
	var fitted_size: Vector2i = Vector2i(maxi(min_size.x, mini(size.x, capacity.x)), maxi(min_size.y, mini(size.y, capacity.y)))
	if size != fitted_size:
		size = fitted_size
	var maximum_position: Vector2i = maximum_edge - size
	position = Vector2i(clampi(position.x, minimum_position.x, maxi(minimum_position.x, maximum_position.x)), clampi(position.y, minimum_position.y, maxi(minimum_position.y, maximum_position.y)))


func _add_button(parent: Container, text: String, node_name: String, action: Callable) -> void:
	var button: Button = Button.new()
	button.name = node_name
	button.text = text
	button.pressed.connect(action)
	parent.add_child(button)


func _clear_children(parent: Node) -> void:
	for child: Node in parent.get_children():
		_retire(child)
		parent.remove_child(child)
		child.queue_free()


func _retire(node: Node) -> void:
	node.set_meta("retired", true)
	for child: Node in node.get_children():
		_retire(child)


func _refresh(rebuild_general: bool = false) -> void:
	if _closed:
		return
	var dimensions: Vector2i = Vector2i(maxi(1, int(_preview.size.x)), maxi(1, int(_preview.size.y)))
	var result: Dictionary = simulation.laboratory_snapshot(draft, dimensions.x, dimensions.y)
	if not _accept(result) or not _accept_preview(result, dimensions):
		return
	_state = result
	var genes: Array = result["genes"]
	_estimates.text = "%s: %d\n" % [_message("T_GENES"), genes.size()]
	if str(result["estimate_error"]).is_empty():
		_estimates.text += "%s %d\n%s %d\n%s %d" % [_message("T_SEGMENTS").strip_edges(), result["estimated_segments"], _message("T_ENERGY_TO_REPRODUCE").strip_edges(), result["estimated_energy"], _message("T_LIFE_EXPECTANCY").strip_edges(), result["estimated_life"]]
	else:
		_estimates.text += str(result["estimate_error"])
	if not str(result["preview_error"]).is_empty() and not genes.is_empty():
		_estimates.text += "\n" + _message("R_LAB_PREVIEW_ERROR") % str(result["preview_error"])
	var warnings: PackedStringArray = result["editor_warnings"]
	if not warnings.is_empty():
		_estimates.text += "\n" + _message("R_LAB_IMPORTED_WARNING") % [warnings.size(), warnings[0]]
		_estimates.tooltip_text = "\n".join(warnings)
	else:
		_estimates.tooltip_text = ""
	_gene_list.clear()
	for index: int in genes.size():
		var gene: Dictionary = genes[index]
		_gene_list.add_item("%d  %s" % [index + 1, _color_name(gene["color"])])
	_selected_gene = mini(_selected_gene, genes.size() - 1)
	if _selected_gene < 0 and not genes.is_empty():
		_selected_gene = 0
	if _selected_gene >= 0:
		_gene_list.select(_selected_gene)
	_rebuild_gene_editor()
	if rebuild_general:
		_rebuild_general()


func _rebuild_general() -> void:
	for field: String in _invalid.keys():
		if field.begins_with("Genome_"):
			_invalid.erase(field)
	_clear_children(_general)
	var properties: Dictionary = _state["properties"]
	for field: String in NUMERIC_PROPERTIES:
		_add_field(_general, _field_name(field), str(properties[field]), _edit_property.bind(field), "Genome_" + field, _step_property.bind(field))
	for field: String in BOOLEAN_PROPERTIES:
		var label: Label = Label.new()
		label.text = _field_name(field)
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		label.custom_minimum_size.x = 160
		_general.add_child(label)
		var toggle: CheckBox = CheckBox.new()
		toggle.name = "Genome_" + field
		toggle.accessibility_name = label.text
		toggle.button_pressed = bool(properties[field])
		toggle.toggled.connect(_toggle_property.bind(field))
		_general.add_child(toggle)


func _rebuild_gene_editor() -> void:
	for field: String in _invalid.keys():
		if field.begins_with("Gene") or field.begins_with("Reaction_"):
			_invalid.erase(field)
	_clear_children(_gene_editor)
	if _selected_gene < 0:
		return
	var gene: Dictionary = _state["genes"][_selected_gene]
	var fields: GridContainer = GridContainer.new()
	fields.columns = 2
	_gene_editor.add_child(fields)
	_add_field(fields, _message("T_LENGTH") + " (2..18)", str(gene["length"]), _edit_gene.bind("length", _selected_gene), "GeneLength", _step_gene.bind("length", _selected_gene))
	_add_field(fields, _message("R_LAB_ANGLE"), str(gene["angle_degrees"]), _edit_gene.bind("angle_degrees", _selected_gene), "GeneAngle", _step_gene.bind("angle_degrees", _selected_gene))
	_add_field(fields, _message("T_BRANCH") + " (-1..999)", str(gene["branch"]), _edit_gene.bind("branch", _selected_gene), "GeneBranch", _step_gene.bind("branch", _selected_gene))
	var color_label: Label = Label.new()
	color_label.text = _message("T_COLOR2")
	fields.add_child(color_label)
	var palette: OptionButton = OptionButton.new()
	palette.name = "GeneColor"
	palette.accessibility_name = color_label.text
	for color_name: String in COLOR_NAMES:
		palette.add_item(_message("T_" + color_name.to_upper().replace(" ", "")))
	var color_index: int = COLOR_VALUES.find(gene["color"]) if gene["color"] != null else -1
	if color_index < 0:
		palette.add_item(_message("R_LAB_PRESERVED") % _color_name(gene["color"]))
		color_index = palette.item_count - 1
	palette.select(color_index)
	palette.item_selected.connect(_color_changed)
	fields.add_child(palette)
	var argb_label: Label = Label.new()
	argb_label.text = _message("R_LAB_ARGB")
	fields.add_child(argb_label)
	var argb: LineEdit = LineEdit.new()
	argb.name = "GeneArgb"
	argb.text = "" if gene["color"] == null else "%08X" % int(gene["color"])
	argb.placeholder_text = _message("R_LAB_PRESERVED") % _message("R_LAB_NO_COLOR") if gene["color"] == null else "AARRGGBB"
	argb.select_all_on_focus = true
	argb.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	argb.accessibility_name = _message("R_LAB_ARGB_ACCESSIBLE")
	argb.accessibility_description = _message("R_LAB_ARGB_DESCRIPTION")
	argb.tooltip_text = argb.accessibility_description
	argb.set_meta("accepted_text", argb.text)
	argb.set_meta("field_label", _message("R_LAB_ARGB"))
	var argb_callback: Callable = _edit_argb.bind(_selected_gene)
	argb.text_submitted.connect(func(_text: String) -> void:
		argb.accept_event()
		_submit(argb, argb_callback, true))
	argb.focus_exited.connect(_submit.bind(argb, argb_callback, true))
	fields.add_child(argb)
	var color_help: Label = Label.new()
	color_help.name = "ArgbHelp"
	color_help.text = _message("R_LAB_ARGB_HELP")
	color_help.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_gene_editor.add_child(color_help)
	var heading: Label = Label.new()
	heading.text = _message("R_LAB_REACTIONS")
	_gene_editor.add_child(heading)
	var reactions: GridContainer = GridContainer.new()
	reactions.columns = 2
	_gene_editor.add_child(reactions)
	var values: PackedInt32Array = gene["reactions"]
	for reaction: String in REACTIONS:
		_add_field(reactions, _reaction_name(reaction), str(values[STORAGE_REACTIONS.find(reaction)]), _edit_gene.bind("reaction_" + reaction, _selected_gene), "Reaction_" + reaction, _step_gene.bind("reaction_" + reaction, _selected_gene))


func _add_field(parent: Container, label_text: String, value: String, callback: Callable, node_name: String, step: Callable) -> void:
	var label: Label = Label.new()
	label.text = label_text
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.custom_minimum_size.x = 160
	parent.add_child(label)
	var input: LineEdit = LineEdit.new()
	input.name = node_name
	input.accessibility_name = label.text
	input.text = value
	input.custom_minimum_size.x = 100
	input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	input.select_all_on_focus = true
	input.set_meta("accepted_text", value)
	input.set_meta("field_label", label.text)
	input.text_submitted.connect(func(_text: String) -> void:
		input.accept_event()
		_submit(input, callback))
	input.focus_exited.connect(_submit.bind(input, callback))
	var row: HBoxContainer = HBoxContainer.new()
	row.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	parent.add_child(row)
	row.add_child(input)
	for direction: int in [-1, 1]:
		var button: Button = Button.new()
		button.name = node_name + ("Decrement" if direction < 0 else "Increment")
		button.text = "-" if direction < 0 else "+"
		button.tooltip_text = _message("R_LAB_DECREASE" if direction < 0 else "R_LAB_INCREASE") % label_text
		button.pressed.connect(step.bind(direction))
		row.add_child(button)
	input.gui_input.connect(func(event: InputEvent) -> void:
		if event is InputEventKey and event.pressed and event.keycode in [KEY_UP, KEY_DOWN]:
			step.call(1 if event.keycode == KEY_UP else -1)
			input.accept_event())


func _step_property(direction: int, field: String) -> void:
	var value: int = int(_state["properties"][field]) + direction
	if _accept(simulation.laboratory_set_property(draft, field, value)):
		_invalid.erase("Genome_" + field)
		_refresh(true)


func _step_gene(direction: int, field: String, gene_index: int) -> void:
	if _accept(draft.step_gene_value(gene_index, field, direction)):
		_refresh()


func _submit(input: LineEdit, callback: Callable, exact_text: bool = false) -> void:
	if _closed or input.is_queued_for_deletion() or input.get_meta("retired", false):
		return
	if input.text == str(input.get_meta("accepted_text")):
		_invalid.erase(str(input.name))
		input.remove_theme_color_override("font_color")
		input.tooltip_text = ""
		if _invalid.is_empty():
			_error.text = ""
		return
	var text: String = input.text if exact_text else input.text.strip_edges()
	var result: Dictionary = callback.call(text)
	if _accept(result):
		input.set_meta("accepted_text", input.text)
		input.remove_theme_color_override("font_color")
		input.tooltip_text = ""
		_invalid.erase(str(input.name))
		_refresh()
	else:
		_invalid[str(input.name)] = str(input.get_meta("field_label"))
		input.add_theme_color_override("font_color", Color("ffb2a9"))
		input.tooltip_text = str(result["error"])


func _edit_property(text: String, field: String) -> Dictionary:
	if not _valid_integer(text):
		return {"ok": false, "error": _message("R_LAB_INTEGER") % _field_name(field)}
	return simulation.laboratory_set_property(draft, field, text.to_int())


func _toggle_property(value: bool, field: String) -> void:
	if _accept(simulation.laboratory_set_property(draft, field, value)):
		_refresh()


func _edit_gene(text: String, field: String, gene_index: int) -> Dictionary:
	if field == "length":
		if not text.is_valid_float():
			return {"ok": false, "error": _message("R_LAB_LENGTH_ERROR")}
		return draft.set_gene_value(gene_index, field, text.to_float())
	if not _valid_integer(text):
		return {"ok": false, "error": _message("R_LAB_INTEGER") % _field_name(field)}
	return draft.set_gene_value(gene_index, field, text.to_int())


func _edit_argb(text: String, gene_index: int) -> Dictionary:
	if text.length() != 8:
		return {"ok": false, "error": _message("R_LAB_ARGB_ERROR")}
	for index: int in text.length():
		var character: int = text.unicode_at(index)
		if not (character >= 48 and character <= 57 or character >= 65 and character <= 70 or character >= 97 and character <= 102):
			return {"ok": false, "error": _message("R_LAB_ARGB_ERROR")}
	return draft.set_gene_value(gene_index, "color", text.hex_to_int())


func _valid_integer(text: String) -> bool:
	if not text.is_valid_int():
		return false
	var normalized: String = text.trim_prefix("+")
	var negative: bool = normalized.begins_with("-")
	normalized = normalized.trim_prefix("-")
	while normalized.length() > 1 and normalized.begins_with("0"):
		normalized = normalized.substr(1)
	var limit: String = "2147483648" if negative else "2147483647"
	return normalized.length() < limit.length() or (normalized.length() == limit.length() and normalized <= limit)


func _color_name(value: Variant) -> String:
	if value == null:
		return _message("R_LAB_NO_COLOR")
	var index: int = COLOR_VALUES.find(int(value))
	return _message("T_" + COLOR_NAMES[index].to_upper().replace(" ", "")) if index >= 0 else "ARGB %08X" % int(value)


func _color_changed(index: int) -> void:
	if index >= COLOR_VALUES.size():
		return
	if _accept(draft.set_gene_value(_selected_gene, "palette_color", index)):
		_refresh()


func _select_gene(index: int) -> void:
	_selected_gene = index
	_rebuild_gene_editor()


func _add_gene() -> void:
	_selected_gene = _state["genes"].size()
	if _accept(draft.edit_rows("insert", _selected_gene)):
		_refresh()


func _insert_gene() -> void:
	_selected_gene = maxi(0, _selected_gene)
	if _accept(draft.edit_rows("insert", _selected_gene)):
		_refresh()


func _delete_gene() -> void:
	if _selected_gene >= 0 and _accept(draft.edit_rows("remove", _selected_gene)):
		_refresh()


func _clear() -> void:
	if _accept(draft.edit_rows("clear", 0)):
		_refresh()


func _refresh_preview() -> void:
	if is_node_ready() and not _closed:
		var dimensions: Vector2i = Vector2i(maxi(1, int(_preview.size.x)), maxi(1, int(_preview.size.y)))
		var result: Dictionary = simulation.laboratory_snapshot(draft, dimensions.x, dimensions.y)
		if _accept(result):
			_accept_preview(result, dimensions)


func _accept_preview(result: Dictionary, dimensions: Vector2i) -> bool:
	if not result.get("preview") is Dictionary or not result.get("preview_error") is String:
		_error.text = _message("R_LAB_PREVIEW_DATA_ERROR")
		return false
	if not _preview.set_raster(result["preview"], dimensions):
		_error.text = _message("R_LAB_PREVIEW_IMAGE_ERROR")
		return false
	if not result["preview_error"].is_empty():
		_error.text = result["preview_error"]
	return true


func _import() -> void:
	var language: String = str(simulation.settings_snapshot()["language"])
	_import_dialog.title = MainLabels.message("T_IMPORT_GENETIC_CODE", language)
	_import_dialog.filters = PackedStringArray([FileFilterLabels.filter_entry("*.bgg", "T_BIOGENESIS_GENETIC_CODE_FILES", language), FileFilterLabels.filter_entry("*.bgg", "T_BIOGENESIS_GENETIC_CODE_FILES", language).replace("*.bgg", "*.xml"), FileFilterLabels.filter_entry("*", "T_ALL_FILES", language)])
	_import_dialog.popup_centered_clamped(Vector2i(760, 520))


func _import_selected(path: String) -> void:
	if _accept(draft.import_file(path)):
		_invalid.clear()
		_refresh(true)


func _accept(result: Dictionary) -> bool:
	var accepted: bool = bool(result.get("ok", false))
	_error.text = "" if accepted else str(result.get("error", _message("R_LAB_OPERATION_FAILED")))
	return accepted


func _commit() -> void:
	gui_release_focus()
	if not _invalid.is_empty():
		_error.text = _message("R_LAB_CORRECT_ENTRIES") % ", ".join(_invalid.values())
		return
	var result: Dictionary = simulation.commit_laboratory(draft)
	if _accept(result):
		_closed = true
		committed.emit(result)
		queue_free()


func _cancel() -> void:
	if not _closed:
		_closed = true
		draft.cancel()
	queue_free()


func _on_window_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and event.keycode == KEY_ESCAPE and not _import_dialog.visible:
		_cancel()
		set_input_as_handled()


func _unhandled_key_input(event: InputEvent) -> void:
	if _closed or _import_dialog.visible or not event is InputEventKey:
		return
	if not event.pressed or event.echo or event.alt_pressed or event.ctrl_pressed or event.meta_pressed or event.shift_pressed:
		return
	if gui_get_focus_owner() is BaseButton:
		return
	if event.keycode in [KEY_ENTER, KEY_KP_ENTER]:
		_commit()
		set_input_as_handled()


func _message(key: String) -> String:
	return MainLabels.message(key, _language)


func _reaction_name(reaction: String) -> String:
	return _message(str(REACTION_LABELS.get(reaction, "T_" + reaction.to_upper())))


func _field_name(field: String) -> String:
	if PROPERTY_LABELS.has(field):
		return _message(str(PROPERTY_LABELS[field]))
	if field.begins_with("reaction_"):
		return _reaction_name(field.trim_prefix("reaction_"))
	match field:
		"length": return _message("T_LENGTH")
		"angle_degrees": return _message("R_LAB_ANGLE")
		"branch": return _message("T_BRANCH")
	return field
