# Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
# SPDX-License-Identifier: GPL-2.0-or-later

extends Window

signal closed
signal copied(result: Dictionary)

const GenomePreview = preload("res://ui/genome_preview.gd")
const StatisticsGraph = preload("res://ui/statistics_graph.gd")
const Labels = preload("res://ui/statistics_labels.gd")
const MainLabels = preload("res://ui/main_labels.gd")
const CURRENT_FIELDS: Array = [
	["CurrentTime", "T_TIME", "current_time", true],
	["CurrentOxygen", "T_OXYGEN2", "current_oxygen", false],
	["CurrentPopulation", "T_POPULATION2", "current_population", true],
	["CurrentCarbonDioxide", "T_CARBON_DIOXIDE2", "current_carbon_dioxide", false],
	["CurrentMethane", "T_METHANE2", "current_methane", false],
	["CurrentCorpses", "T_REMAINS_OF_BEINGS", "current_corpses", true],
]
const POPULATION_FIELDS: Array = [
	["AveragePopulation", "T_AVERAGE_POPULATION", "average_population", false],
	["AverageBirths", "T_AVERAGE_BIRTH_RATE", "average_births", false],
	["AverageDeaths", "T_AVERAGE_MORTALITY_RATE", "average_deaths", false],
	["AverageInfections", "T_AVERAGE_INFECTIONS_RATE", "average_infections", false],
	["CreatedOrganisms", "T_GENERATED_ORGANISMS", "created_organisms", false],
	["MaximumPopulation", "T_MAXIMUM_POPULATION", "max_population", false],
	["MaximumPopulationTime", "T_AT_TIME", "max_population_time", false],
	["MinimumPopulation", "T_MINIMUM_POPULATION", "min_population", false],
	["MinimumPopulationTime", "T_AT_TIME", "min_population_time", false],
	["MassExtinctions", "T_MASS_EXTINTIONS", "mass_extinctions", false],
]
const ATMOSPHERE_FIELDS: Array = [
	["MaximumCarbonDioxide", "T_MAXIMUM_CARBON_DIOXIDE", "max_carbon_dioxide", false],
	["MaximumCarbonDioxideTime", "T_AT_TIME", "max_carbon_dioxide_time", false],
	["MinimumCarbonDioxide", "T_MINIMUM_CARBON_DIOXIDE", "min_carbon_dioxide", false],
	["MinimumCarbonDioxideTime", "T_AT_TIME", "min_carbon_dioxide_time", false],
	["MaximumMethane", "T_MAXIMUM_METHANE", "max_methane", false],
	["MaximumMethaneTime", "T_AT_TIME", "max_methane_time", false],
	["MinimumMethane", "T_MINIMUM_METHANE", "min_methane", false],
	["MinimumMethaneTime", "T_AT_TIME", "min_methane_time", false],
	["MaximumOxygen", "T_MAXIMUM_OXYGEN", "max_oxygen", false],
	["MaximumOxygenTime", "T_AT_TIME", "max_oxygen_time", false],
	# Source StatisticsWindow uses the minimum oxygen timestamp for this label
	["MinimumOxygen", "T_MINIMUM_OXYGEN", "min_oxygen_time", false],
	["MinimumOxygenTime", "T_AT_TIME", "min_oxygen_time", false],
]
const RECORD_TITLES: Array[String] = [
	"T_ALIVE_BEING_HAVING_THE_MOST_CHILDREN", "T_BEING_HAVING_THE_MOST_CHILDREN",
	"T_ALIVE_BEING_HAVING_THE_MOST_VICTIMS", "T_BEING_HAVING_THE_MOST_VICTIMS",
	"T_ALIVE_BEING_HAVING_THE_MOST_INFECTED", "T_BEING_HAVING_THE_MOST_INFECTED",
]
const RECORD_COUNTS: Array[String] = ["T_NUMBER_OF_CHILDREN", "T_NUMBER_OF_VICTIMS", "T_NUMBER_OF_INFECTED"]
const GRAPH_NAMES: Array[String] = ["T_DEATHS", "T_BIRTHS", "T_POPULATION", "T_OXYGEN", "T_CARBON_DIOXIDE", "T_METHANE"]

var simulation: BgrSimulation
var initial_packet: Dictionary = {}
var _packet: Dictionary = {}
var _language: String = "en"
var _content: HBoxContainer
var _error: Label
var _update_button: Button
var _close_button: Button
var _previews: Array[GenomePreview] = []
var _copy_buttons: Array[Button] = []
var _closing: bool = false


class ColorDistribution:
	extends Control
	var colors: PackedInt64Array = PackedInt64Array()
	var rectangles: PackedInt32Array = PackedInt32Array()

	func _draw() -> void:
		draw_rect(Rect2(Vector2.ZERO, size), Color("182735"))
		for index: int in range(rectangles.size() >> 2):
			var offset: int = index * 4
			if rectangles[offset + 2] <= 0 or rectangles[offset + 3] <= 0:
				continue
			var argb: int = colors[index]
			var color: Color = Color8((argb >> 16) & 255, (argb >> 8) & 255, argb & 255, (argb >> 24) & 255)
			draw_rect(Rect2(rectangles[offset], rectangles[offset + 1], rectangles[offset + 2], rectangles[offset + 3]), color)


func _ready() -> void:
	transient = true
	exclusive = false
	min_size = Vector2i(800, 540)
	size = Vector2i(1180, 790)
	close_requested.connect(close_statistics)
	window_input.connect(_on_window_input)
	var margin: MarginContainer = MarginContainer.new()
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for side: String in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 12)
	add_child(margin)
	var column: VBoxContainer = VBoxContainer.new()
	column.add_theme_constant_override("separation", 10)
	margin.add_child(column)
	_error = Label.new()
	_error.name = "StatisticsError"
	_error.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_error.add_theme_color_override("font_color", Color("ffb2a9"))
	_error.hide()
	column.add_child(_error)
	var scroll: ScrollContainer = ScrollContainer.new()
	scroll.name = "StatisticsScroll"
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column.add_child(scroll)
	_content = HBoxContainer.new()
	_content.add_theme_constant_override("separation", 22)
	scroll.add_child(_content)
	var buttons: HBoxContainer = HBoxContainer.new()
	buttons.alignment = BoxContainer.ALIGNMENT_END
	column.add_child(buttons)
	_update_button = Button.new()
	_update_button.name = "UpdateStatistics"
	_update_button.pressed.connect(_update_statistics)
	buttons.add_child(_update_button)
	_close_button = Button.new()
	_close_button.name = "CloseStatistics"
	_close_button.pressed.connect(close_statistics)
	buttons.add_child(_close_button)
	if _apply_packet(initial_packet):
		title = Labels.message("T_STATISTICS", _language)
	popup_centered_clamped(size)
	_update_button.grab_focus()


func _process(_delta: float) -> void:
	if not _closing and visible and simulation != null:
		_refresh_previews()


func _apply_packet(packet: Dictionary) -> bool:
	if not bool(packet.get("ok", false)):
		_show_error(str(packet.get("error", "Statistics are unavailable.")))
		return false
	if not _valid_packet(packet):
		_show_error("The simulation returned invalid statistics data.")
		return false
	_packet = packet.duplicate(true)
	_language = str(packet["language"])
	_error.hide()
	_update_button.text = Labels.message("T_UPDATE", _language)
	_update_button.accessibility_name = _update_button.text
	_close_button.text = Labels.message("T_CLOSE", _language)
	_close_button.accessibility_name = _close_button.text
	for node: Node in _content.get_children():
		_content.remove_child(node)
		node.queue_free()
	_previews.clear()
	_copy_buttons.clear()
	_build_content()
	_refresh_previews()
	return true


func _valid_packet(packet: Dictionary) -> bool:
	for field: String in ["language", "raw", "display", "records", "graphs", "palette_counts", "palette_colors", "palette_rectangles"]:
		if not packet.has(field):
			return false
	if not packet["language"] is String or not packet["raw"] is Dictionary or not packet["display"] is Dictionary:
		return false
	var raw: Dictionary = packet["raw"]
	var display: Dictionary = packet["display"]
	for fields: Array in [CURRENT_FIELDS, POPULATION_FIELDS, ATMOSPHERE_FIELDS]:
		for field: Array in fields:
			var key: String = str(field[2])
			if not raw.has(key) or not (raw[key] is int or raw[key] is float) or not display.get(key) is String:
				return false
	if not packet["records"] is Array or packet["records"].size() != 6:
		return false
	for index: int in range(6):
		var record: Variant = packet["records"][index]
		if not record is Dictionary or record.get("kind") != index or not record.get("count") is int or not record.get("present") is bool:
			return false
		if index % 2 == 1 and not record.get("time") is int:
			return false
	if not packet["graphs"] is Array or packet["graphs"].size() != 6:
		return false
	for index: int in range(6):
		var graph: Variant = packet["graphs"][index]
		if not graph is Dictionary or graph.get("kind") != index or not graph.get("argb") is int:
			return false
		if not graph.get("minimum") is float or not graph.get("maximum") is float or not graph.get("points") is PackedInt32Array:
			return false
		var points: PackedInt32Array = graph["points"]
		if points.size() > 200 or points.size() % 2 != 0:
			return false
	if not packet["palette_counts"] is PackedInt32Array or packet["palette_counts"].size() != 46:
		return false
	if not packet["palette_colors"] is PackedInt64Array or packet["palette_colors"].size() != 46:
		return false
	if not packet["palette_rectangles"] is PackedInt32Array:
		return false
	return packet["palette_rectangles"].size() in [0, 184]


func _build_content() -> void:
	var left: VBoxContainer = VBoxContainer.new()
	left.custom_minimum_size.x = 640
	_content.add_child(left)
	var current: VBoxContainer = _group(left, "CurrentState", "T_CURRENT_STATE")
	_field_rows(current, CURRENT_FIELDS)
	_label(current, "ColorProportion", Labels.message("T_COLOR_PROPORTION", _language))
	var palette: ColorDistribution = ColorDistribution.new()
	palette.name = "ColorDistribution"
	palette.custom_minimum_size = Vector2(640, 25)
	palette.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	palette.colors = _packet["palette_colors"]
	palette.rectangles = _packet["palette_rectangles"]
	palette.mouse_filter = Control.MOUSE_FILTER_IGNORE
	current.add_child(palette)
	var notable: VBoxContainer = _group(left, "NotableBeings", "T_REMARKABLE_ORGANISMS")
	_build_records(notable)
	var right: VBoxContainer = VBoxContainer.new()
	right.custom_minimum_size.x = 360
	_content.add_child(right)
	var history: VBoxContainer = _group(right, "WorldHistory", "T_WORLD_HISTORY")
	var population: VBoxContainer = _group(history, "Population", "T_POPULATION")
	_graph(population, "PopulationGraph", 0)
	_field_rows(population, POPULATION_FIELDS)
	var atmosphere: VBoxContainer = _group(history, "Atmosphere", "T_ATMOSPHERE")
	_graph(atmosphere, "AtmosphereGraph", 3)
	_field_rows(atmosphere, ATMOSPHERE_FIELDS)


func _group(parent: VBoxContainer, node_name: String, key: String) -> VBoxContainer:
	var group: VBoxContainer = VBoxContainer.new()
	group.name = node_name
	group.add_theme_constant_override("separation", 5)
	parent.add_child(group)
	var heading: Label = _label(group, node_name + "Title", Labels.message(key, _language))
	heading.add_theme_font_size_override("font_size", 17)
	return group


func _field_rows(parent: VBoxContainer, fields: Array) -> void:
	var raw: Dictionary = _packet["raw"]
	var display: Dictionary = _packet["display"]
	for field: Array in fields:
		var value: String = str(raw[field[2]]) if bool(field[3]) else str(display[field[2]])
		_label(parent, str(field[0]), Labels.message(str(field[1]), _language) + value)


func _graph(parent: VBoxContainer, node_name: String, first: int) -> void:
	var series: Array[Dictionary] = []
	var names: PackedStringArray = PackedStringArray()
	for index: int in range(first, first + 3):
		series.append(_packet["graphs"][index])
		names.append(Labels.message(GRAPH_NAMES[index], _language))
	var graph: StatisticsGraph = StatisticsGraph.new()
	graph.name = node_name
	graph.configure(series, names)
	parent.add_child(graph)


func _build_records(parent: VBoxContainer) -> void:
	var grid: GridContainer = GridContainer.new()
	grid.name = "RecordGrid"
	grid.columns = 2
	grid.add_theme_constant_override("h_separation", 18)
	grid.add_theme_constant_override("v_separation", 7)
	parent.add_child(grid)
	for pair: int in range(3):
		for index: int in range(pair * 2, pair * 2 + 2):
			_label(grid, "Record%dTitle" % index, Labels.message(RECORD_TITLES[index], _language))
		for index: int in range(pair * 2, pair * 2 + 2):
			var record_column: VBoxContainer = VBoxContainer.new()
			grid.add_child(record_column)
			var preview: GenomePreview = GenomePreview.new()
			preview.name = "Record%dPreview" % index
			preview.custom_minimum_size = Vector2(100, 100)
			preview.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
			preview.copy_enabled = true
			preview.focus_mode = Control.FOCUS_ALL
			preview.accessibility_name = Labels.message(RECORD_TITLES[index], _language)
			preview.copy_requested.connect(_copy_record.bind(index))
			preview.gui_input.connect(_preview_input.bind(index))
			record_column.add_child(preview)
			preview.set_copy_caption(MainLabels.message("T_COPY", _language))
			_previews.append(preview)
			var copy_button: Button = Button.new()
			copy_button.name = "Record%dCopy" % index
			copy_button.text = MainLabels.message("T_COPY", _language)
			copy_button.accessibility_name = copy_button.text + " " + Labels.message(RECORD_TITLES[index], _language)
			copy_button.custom_minimum_size = Vector2(48, 48)
			copy_button.disabled = true
			copy_button.pressed.connect(_copy_record.bind(index))
			record_column.add_child(copy_button)
			_copy_buttons.append(copy_button)
		for index: int in range(pair * 2, pair * 2 + 2):
			var record: Dictionary = _packet["records"][index]
			_label(grid, "Record%dCount" % index, Labels.message(RECORD_COUNTS[pair], _language) + str(record["count"]))
		var spacer: Control = Control.new()
		grid.add_child(spacer)
		var historical: Dictionary = _packet["records"][pair * 2 + 1]
		_label(grid, "Record%dTime" % (pair * 2 + 1), Labels.message("T_TIME", _language) + str(historical["time"]))


func _label(parent: Container, node_name: String, caption: String) -> Label:
	var label: Label = Label.new()
	label.name = node_name
	label.text = caption
	label.accessibility_name = caption
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.custom_minimum_size.x = 220
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	parent.add_child(label)
	return label


func _update_statistics() -> void:
	if not _closing:
		_apply_packet(simulation.statistics_update())


func _refresh_previews() -> void:
	if _previews.size() != 6:
		return
	var result: Dictionary = simulation.statistics_previews()
	if not bool(result.get("open", false)):
		close_statistics()
		return
	if not bool(result.get("ok", false)) or not result.get("previews") is Array or result["previews"].size() != 6:
		_show_error(str(result.get("error", "The simulation returned invalid statistics previews.")))
		return
	for index: int in range(6):
		var item: Variant = result["previews"][index]
		if not item is Dictionary or not item.get("image") is Dictionary or not item.get("error") is String or not item.get("present") is bool:
			_show_error("The simulation returned invalid record preview data.")
			return
		if not GenomePreview.valid_raster(item["image"]):
			_show_error("The simulation returned invalid record preview pixels.")
			return
		var image: Dictionary = item["image"]
		if (not bool(item["present"]) and not image.is_empty()) or (bool(item["present"]) and str(item["error"]).is_empty() and image.is_empty()):
			_show_error("The simulation returned inconsistent record preview data.")
			return
	for index: int in range(6):
		var item: Dictionary = result["previews"][index]
		if not _previews[index].set_raster(item["image"]):
			_show_error("Cannot create the statistics preview texture.")
			return
		_copy_buttons[index].disabled = not bool(item["present"])
		_previews[index].tooltip_text = str(item["error"]) if not str(item["error"]).is_empty() else Labels.message("R_COPY_GENOTYPE_HINT", _language)
		if not str(item["error"]).is_empty():
			_show_error(str(item["error"]))


func _copy_record(index: int) -> void:
	if _closing:
		return
	var result: Dictionary = simulation.statistics_copy_record(index)
	if bool(result.get("ok", false)):
		copied.emit(result)
	else:
		_show_error(str(result.get("error", "Cannot copy this record.")))


func _preview_input(event: InputEvent, index: int) -> void:
	if event is InputEventKey and event.pressed and not event.echo and event.is_action("ui_accept"):
		_copy_record(index)
		get_viewport().set_input_as_handled()


func _show_error(message: String) -> void:
	_error.text = message
	_error.show()


func close_statistics() -> void:
	if _closing:
		return
	_closing = true
	set_process(false)
	if simulation != null:
		simulation.statistics_close()
	simulation = null
	closed.emit()
	queue_free()


func _on_window_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo and event.physical_keycode == KEY_ESCAPE:
		close_statistics()
		get_viewport().set_input_as_handled()


func _exit_tree() -> void:
	if not _closing and simulation != null:
		simulation.statistics_close()
	simulation = null
