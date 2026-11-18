extends Control

signal focus_subject(token: int)

const GenomePreview = preload("res://ui/genome_preview.gd")
const StatisticsLabels = preload("res://ui/statistics_labels.gd")
const MainLabels = preload("res://ui/main_labels.gd")
const KINDS: Array[String] = ["offspring", "kills", "age", "mass", "infections"]
const FIELDS: Array[String] = ["children", "kills", "age_ticks", "mass", "infections"]
const METRIC_LABELS: Array[String] = ["T_CHILDREN", "T_VICTIMS", "T_AGE", "T_MASS", "T_INFECTED"]
const ALL_TIME_METRICS: Array[int] = [0, 1, 4]
const LEADERS_WIDTH: float = 720.0
const ALL_TIME_WIDTH: float = 514.0
const LEADERS_GAP: int = 8
const PORTRAIT_SIZE: float = 100.0
const EDGE_GAP: float = 20.0
const WORDS: Dictionary = {
	"en": ["Living record", "Living leaders", "Current ecology", "No living organisms", "No positive record", "Organism", "256 age ticks", "Source mass", "Count", "All-time records", "Living and dead", "Appearance unavailable"],
	"es": ["Récord vivo", "Líderes vivos", "Ecología actual", "Sin organismos vivos", "Sin récord positivo", "Organismo", "256 pasos de edad", "Masa original", "Cantidad", "Récords históricos", "Vivos y muertos", "Aspecto no disponible"],
	"ca": ["Rècord viu", "Líders vius", "Ecologia actual", "Sense organismes vius", "Sense rècord positiu", "Organisme", "256 passos d'edat", "Massa original", "Quantitat", "Rècords històrics", "Vius i morts", "Aspecte no disponible"],
	"de": ["Lebender Rekord", "Lebende Spitzenreiter", "Aktuelle Ökologie", "Keine lebenden Organismen", "Kein positiver Rekord", "Organismus", "256 Altersschritte", "Quellmasse", "Anzahl", "Allzeitrekorde", "Lebende und Tote", "Aussehen nicht verfügbar"],
	"pt": ["Recorde vivo", "Líderes vivos", "Ecologia atual", "Sem organismos vivos", "Sem recorde positivo", "Organismo", "256 passos de idade", "Massa original", "Contagem", "Recordes históricos", "Vivos e mortos", "Aparência indisponível"],
	"ru": ["Рекорд среди живых", "Лидеры среди живых", "Текущая экология", "Нет живых организмов", "Нет положительного рекорда", "Организм", "256 шагов возраста", "Исходная масса", "Количество", "Исторические рекорды", "Живые и умершие", "Внешний вид недоступен"]
}

var slide_interval_seconds: float = 60.0
var slide_dwell_seconds: float = 8.0
var _panel: PanelContainer
var _card_width: float = 0.0
var _title: Label
var _number: Label
var _body: Label
var _content: HBoxContainer
var _preview: GenomePreview
var _leaders: HBoxContainer
var _leader_previews: Array[GenomePreview] = []
var _leader_labels: Array[Label] = []
var _appearance_unavailable: Array[Label] = []
var _preview_images: Dictionary = {}
var _preview_tokens: Array[int] = []
var _subjects: Dictionary = {}
var _records: Array[Dictionary] = []
var _all_time_records: Array[Dictionary] = []
var _context: Dictionary = {}
var _epoch: int = -1
var _available: bool = false
var _elapsed: float = 0.0
var _dwell: float = 0.0
var _next_mode: int = 0
var _mode: int = 0
var _next_metric: int = 0
var _metric: int = 0
var _focus_token: int = 0
var _language: String = "en"


func _ready() -> void:
	_build()
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	resized.connect(_layout)
	_layout()


## Accept only current live identities and source-defined record values
func set_scene(packet: Dictionary, context: Dictionary) -> bool:
	_build()
	if typeof(packet.get("schema")) != TYPE_INT or packet["schema"] != 1 or typeof(packet.get("ok")) != TYPE_BOOL or not packet["ok"] or typeof(packet.get("world_ready")) != TYPE_BOOL or not packet["world_ready"] or typeof(packet.get("presentation_epoch")) != TYPE_INT or int(packet["presentation_epoch"]) < 0:
		_clear()
		return false
	var subjects_value: Variant = packet.get("subjects")
	var records_value: Variant = packet.get("records")
	if not subjects_value is Array or subjects_value.size() > 8 or not records_value is Array or records_value.size() != 5:
		_clear()
		return false
	var subjects: Dictionary = {}
	for value: Variant in subjects_value:
		if not value is Dictionary or typeof(value.get("token")) != TYPE_INT or int(value["token"]) <= 0 or typeof(value.get("alive")) != TYPE_BOOL or not value["alive"]:
			_clear()
			return false
		var actor: Dictionary = value
		var token: int = actor["token"]
		if subjects.has(token) or typeof(actor.get("id")) != TYPE_INT:
			_clear()
			return false
		for field: String in ["children", "kills", "age_ticks", "age_world_units", "infections"]:
			if typeof(actor.get(field)) != TYPE_INT:
				_clear()
				return false
		if not _finite(actor.get("mass")) or actor["age_world_units"] != (int(actor["age_ticks"]) >> 8):
			_clear()
			return false
		subjects[token] = {"token": token, "id": actor["id"], "children": actor["children"], "kills": actor["kills"], "age_ticks": actor["age_ticks"], "age_world_units": actor["age_world_units"], "mass": actor["mass"], "infections": actor["infections"]}
	var previews: Dictionary = {}
	for value: Dictionary in subjects_value:
		var preview: Variant = value.get("preview", {"requested": false, "error": "", "image": {}})
		if not _valid_preview(preview):
			_clear()
			return false
		if preview["requested"]:
			previews[value["token"]] = preview["image"].duplicate(true) if preview["error"].is_empty() else {}
	var records: Array[Dictionary] = []
	for index: int in range(5):
		var value: Variant = records_value[index]
		var unit: String = "biological_ticks" if index == 2 else ("source_mass" if index == 3 else "count")
		if not value is Dictionary or value.get("kind") != KINDS[index] or value.get("unit") != unit or typeof(value.get("present")) != TYPE_BOOL or typeof(value.get("subject_token")) != TYPE_INT:
			_clear()
			return false
		var record: Dictionary = value
		var token: int = record["subject_token"]
		if index in [2, 3] and record["present"] == subjects.is_empty():
			_clear()
			return false
		if not _finite(record.get("value")) or (index != 3 and typeof(record["value"]) != TYPE_INT):
			_clear()
			return false
		if record["present"]:
			if not subjects.has(token) or record["value"] != subjects[token][FIELDS[index]] or (index in [0, 1, 4] and record["value"] <= 0):
				_clear()
				return false
		elif token != 0 or record["value"] != 0:
			_clear()
			return false
		records.append({"present": record["present"], "subject_token": token, "value": record["value"]})
	var epoch: int = packet["presentation_epoch"]
	if epoch != _epoch:
		dismiss()
		_next_mode = 0
		_next_metric = 0
	_epoch = epoch
	if needs_all_time_records() and packet.has("all_time_records") and not _set_all_time_records(packet["all_time_records"]):
		_clear()
		return false
	for token: int in _preview_images.keys():
		if not subjects.has(token):
			_preview_images.erase(token)
	if _panel.visible:
		for token: int in previews:
			if _mode == 1 or (_mode == 0 and records[_metric]["subject_token"] == token):
				_preview_images[token] = previews[token]
	_subjects = subjects
	_records = records
	_context = {}
	for key: String in ["population", "world_time"]:
		if typeof(context.get(key)) == TYPE_INT and int(context[key]) >= 0:
			_context[key] = context[key]
	var gases: Variant = context.get("gases", {})
	if gases is Dictionary:
		for key: String in ["oxygen", "carbon_dioxide", "methane"]:
			if _finite(gases.get(key)):
				_context[key] = gases[key]
	_language = str(context.get("language", "en"))
	if not WORDS.has(_language):
		_language = "en"
	_available = true
	if _panel.visible:
		if _focus_token != 0 and not _subjects.has(_focus_token):
			dismiss()
		else:
			_render()
	return true


## Advance one passive slide without accumulating work behind modal fences
func advance(delta: float, allowed: bool) -> void:
	if not allowed or not _available:
		dismiss()
		return
	if not is_finite(delta) or delta < 0.0:
		return
	if _panel.visible:
		_dwell += delta
		if _dwell >= clampf(slide_dwell_seconds, 2.0, 60.0):
			dismiss()
	else:
		_elapsed += delta
		if _elapsed >= clampf(slide_interval_seconds, 10.0, 3600.0):
			_mode = _next_mode
			_next_mode = (_next_mode + 1) % 4
			if _mode == 0:
				_metric = _next_metric
				_next_metric = (_next_metric + 1) % 5
			_render()
			_panel.show()
			_dwell = 0.0


func dismiss() -> void:
	_elapsed = 0.0
	_dwell = 0.0
	_focus_token = 0
	_preview_images.clear()
	_preview_tokens.clear()
	_all_time_records.clear()
	if _panel != null and _panel.visible:
		_set_preview(_preview, {})
		for preview: GenomePreview in _leader_previews:
			_set_preview(preview, {})
		for label: Label in _appearance_unavailable:
			label.hide()
		_panel.hide()


## Request bounded native images only for a visible slide or a changed live winner
func needs_previews() -> bool:
	if _panel == null or not _panel.visible:
		return false
	for token: int in _preview_tokens:
		if not _preview_images.has(token):
			return true
	return false


## Request one source snapshot for each visible all-time slide
func needs_all_time_records() -> bool:
	return _panel != null and _panel.visible and _mode == 3 and _all_time_records.is_empty()


func _valid_preview(preview: Variant) -> bool:
	return preview is Dictionary and typeof(preview.get("requested")) == TYPE_BOOL \
		and preview.get("error") is String and preview.get("image") is Dictionary \
		and GenomePreview.valid_raster(preview["image"]) \
		and (preview["requested"] or (preview["image"].is_empty() and preview["error"].is_empty()))


func _set_all_time_records(value: Variant) -> bool:
	if not value is Array or value.size() != ALL_TIME_METRICS.size():
		return false
	var records: Array[Dictionary] = []
	for index: int in range(ALL_TIME_METRICS.size()):
		var record: Variant = value[index]
		if not record is Dictionary or record.get("kind") != KINDS[ALL_TIME_METRICS[index]] or typeof(record.get("count")) != TYPE_INT or typeof(record.get("time")) != TYPE_INT or typeof(record.get("present")) != TYPE_BOOL:
			return false
		var preview: Variant = record.get("preview")
		if not _valid_preview(preview) or record["present"] != preview["requested"]:
			return false
		if record["present"] and preview["image"].is_empty() == preview["error"].is_empty():
			return false
		records.append({"count": record["count"], "time": record["time"], "image": preview["image"].duplicate(true)})
	_all_time_records = records
	return true


func _set_preview(preview: GenomePreview, image: Dictionary, reserve_space: bool = false) -> void:
	if image != preview.get_meta("card_image", {}):
		var ready: bool = preview.set_raster(image) and not image.is_empty()
		if not ready:
			preview.set_raster({})
		preview.set_meta("card_image", image)
		preview.set_meta("card_image_ready", ready)
	preview.visible = bool(preview.get_meta("card_image_ready", false))
	preview.get_parent().visible = preview.visible or reserve_space


func current_subject_token() -> int:
	return _focus_token if _panel != null and _panel.visible else 0


## Let an external accessible action request focus without capturing world input
func focus_current_subject() -> void:
	if _panel != null and _panel.visible and _subjects.has(_focus_token):
		focus_subject.emit(_focus_token)
		dismiss()


func _input(event: InputEvent) -> void:
	if event is InputEventMouseMotion or event is InputEventMouseButton or event is InputEventScreenTouch or event is InputEventScreenDrag or event is InputEventKey or event is InputEventJoypadButton or event is InputEventJoypadMotion:
		dismiss()


func _clear() -> void:
	dismiss()
	_available = false
	_subjects.clear()
	_records.clear()
	_context.clear()
	_epoch = -1


func _finite(value: Variant) -> bool:
	return (typeof(value) == TYPE_INT or typeof(value) == TYPE_FLOAT) and is_finite(float(value))


func _word(index: int) -> String:
	return str(WORDS[_language][index])


func _metric_label(index: int) -> String:
	return MainLabels.message(METRIC_LABELS[index], _language).strip_edges().trim_suffix(":")


func _value(index: int, record: Dictionary) -> String:
	if not record["present"]:
		return "0"
	if index == 3:
		return MainLabels.format_number(float(record["value"]), _language)
	if index == 2:
		return str(_subjects[record["subject_token"]]["age_world_units"])
	return str(record["value"])


func _render() -> void:
	_focus_token = 0
	_preview_tokens.clear()
	_content.visible = _mode != 3 and (_mode != 1 or _subjects.is_empty())
	_leaders.visible = (_mode == 1 and not _subjects.is_empty()) or (_mode == 3 and not _all_time_records.is_empty())
	for index: int in range(_leaders.get_child_count()):
		var entry: Control = _leaders.get_child(index) as Control
		entry.visible = _mode != 3 or index < ALL_TIME_METRICS.size()
	if _mode == 0:
		var record: Dictionary = _records[_metric]
		_title.text = _word(0) + " - " + _metric_label(_metric)
		_number.text = _value(_metric, record)
		_body.text = _word(3) if _subjects.is_empty() else _word(4)
		if record["present"]:
			_focus_token = record["subject_token"]
			_preview_tokens.append(_focus_token)
			_body.text = _word(5) + " #" + str(_subjects[_focus_token]["id"])
			_body.text += "\n" + _word(6 if _metric == 2 else (7 if _metric == 3 else 8))
		_set_preview(_preview, _preview_images.get(_focus_token, {}))
	elif _mode == 1:
		_title.text = _word(1)
		_number.text = ""
		_body.text = _word(3) if _subjects.is_empty() else ""
		for index: int in range(5):
			var record: Dictionary = _records[index]
			var identity: String = " #" + str(_subjects[record["subject_token"]]["id"]) if record["present"] else ""
			var units: String = "\n" + _word(6 if index == 2 else 7) if index in [2, 3] else ""
			_leader_labels[index].text = _metric_label(index) + "\n" + _value(index, record) + identity + units
			var token: int = record["subject_token"] if record["present"] else 0
			if token != 0 and not _preview_tokens.has(token):
				_preview_tokens.append(token)
			_set_preview(_leader_previews[index], _preview_images.get(token, {}), true)
		_set_preview(_preview, {})
	elif _mode == 3:
		_title.text = _word(9) + " - " + _word(10)
		var time_label: String = StatisticsLabels.message("T_TIME", _language).strip_edges()
		for index: int in range(_all_time_records.size()):
			var record: Dictionary = _all_time_records[index]
			_leader_labels[index].text = _metric_label(ALL_TIME_METRICS[index]) + "\n" + str(record["count"]) + "\n" + time_label + " " + str(record["time"])
			_set_preview(_leader_previews[index], record["image"], true)
			_appearance_unavailable[index].text = _word(11)
			_appearance_unavailable[index].visible = not _leader_previews[index].visible
	else:
		_set_preview(_preview, {})
		_title.text = _word(2)
		_number.text = str(_context["population"]) if _context.has("population") else ""
		var rows: PackedStringArray = []
		if _context.has("population"):
			rows.append(StatisticsLabels.message("T_POPULATION", _language))
		if _context.has("world_time"):
			rows.append(StatisticsLabels.message("T_TIME", _language) + " " + str(_context["world_time"]))
		for entry: Array in [["oxygen", "T_OXYGEN"], ["carbon_dioxide", "T_CARBON_DIOXIDE"], ["methane", "T_METHANE"]]:
			if _context.has(entry[0]):
				rows.append(StatisticsLabels.message(entry[1], _language) + ": " + MainLabels.format_number(float(_context[entry[0]]), _language))
		_body.text = "\n".join(rows)
	_number.visible = not _number.text.is_empty()
	_layout()


func _build() -> void:
	if _panel != null:
		return
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	focus_mode = Control.FOCUS_NONE
	_panel = PanelContainer.new()
	_panel.name = "LivingCard"
	_panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_panel.clip_contents = true
	var style: StyleBoxFlat = StyleBoxFlat.new()
	style.bg_color = Color(0.035, 0.055, 0.065, 0.86)
	style.set_corner_radius_all(18)
	style.content_margin_left = 18.0
	style.content_margin_right = 18.0
	style.content_margin_top = 16.0
	style.content_margin_bottom = 16.0
	_panel.add_theme_stylebox_override("panel", style)
	add_child(_panel)
	_panel.minimum_size_changed.connect(_fit_panel)
	_panel.resized.connect(_position_panel)
	var column: VBoxContainer = VBoxContainer.new()
	column.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.add_theme_constant_override("separation", 8)
	_panel.add_child(column)
	_title = _label(21)
	_number = _label(42)
	_body = _label(17)
	column.add_child(_title)
	_content = HBoxContainer.new()
	_content.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_content.add_theme_constant_override("separation", 12)
	column.add_child(_content)
	_preview = _make_preview(_content)
	var text: VBoxContainer = VBoxContainer.new()
	text.mouse_filter = Control.MOUSE_FILTER_IGNORE
	text.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_content.add_child(text)
	text.add_child(_number)
	text.add_child(_body)
	_leaders = HBoxContainer.new()
	_leaders.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_leaders.add_theme_constant_override("separation", LEADERS_GAP)
	_leaders.hide()
	column.add_child(_leaders)
	for index: int in range(5):
		var entry: VBoxContainer = VBoxContainer.new()
		entry.mouse_filter = Control.MOUSE_FILTER_IGNORE
		entry.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		entry.add_theme_constant_override("separation", 6)
		_leaders.add_child(entry)
		var preview: GenomePreview = _make_preview(entry)
		var holder: Control = preview.get_parent() as Control
		holder.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
		_leader_previews.append(preview)
		if index < ALL_TIME_METRICS.size():
			var missing: Label = _label(14)
			missing.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
			missing.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
			holder.add_child(missing)
			missing.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
			missing.hide()
			_appearance_unavailable.append(missing)
		var label: Label = _label(15)
		label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		if index == 2:
			label.add_theme_font_size_override("font_size", 14)
		entry.add_child(label)
		_leader_labels.append(label)
	_panel.hide()


func _make_preview(parent: Control) -> GenomePreview:
	var holder: Control = Control.new()
	holder.mouse_filter = Control.MOUSE_FILTER_IGNORE
	holder.custom_minimum_size = Vector2(PORTRAIT_SIZE, PORTRAIT_SIZE)
	holder.clip_contents = true
	parent.add_child(holder)
	var preview: GenomePreview = GenomePreview.new()
	preview.fit_visible_content = true
	preview.size = holder.custom_minimum_size
	holder.add_child(preview)
	return preview


func _label(font_size: int) -> Label:
	var label: Label = Label.new()
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.add_theme_color_override("font_color", Color(0.94, 0.97, 0.98))
	label.add_theme_font_size_override("font_size", font_size)
	return label


func _layout() -> void:
	if _panel == null:
		return
	var padding: float = _panel.get_theme_stylebox("panel").get_minimum_size().x
	var limit: float = 360.0
	if _leaders.visible:
		limit = ALL_TIME_WIDTH if _mode == 3 else LEADERS_WIDTH
	limit = minf(limit, maxf(0.0, size.x - EDGE_GAP * 2.0))
	var content_width: float = _text_width(_title)
	if _leaders.visible:
		var columns: int = ALL_TIME_METRICS.size() if _mode == 3 else _leader_labels.size()
		var row_width: float = PORTRAIT_SIZE * columns + LEADERS_GAP * (columns - 1)
		var extra_width: float = 0.0
		var extra_by_column: Array[float] = []
		for index: int in range(columns):
			var extra: float = maxf(0.0, _text_width(_leader_labels[index]) - PORTRAIT_SIZE)
			extra_by_column.append(extra)
			extra_width += extra
		var fraction: float = clampf((limit - padding - row_width) / maxf(1.0, extra_width), 0.0, 1.0)
		for index: int in range(columns):
			_leader_labels[index].custom_minimum_size.x = PORTRAIT_SIZE + floorf(extra_by_column[index] * fraction)
		content_width = maxf(content_width, row_width + extra_width * fraction)
	elif _content.visible:
		var stats_width: float = maxf(_text_width(_number), _text_width(_body))
		if _preview.get_parent().visible:
			stats_width += PORTRAIT_SIZE + _content.get_theme_constant("separation")
		content_width = maxf(content_width, stats_width)
	_card_width = minf(ceilf(content_width + padding), limit)
	_fit_panel()
	_position_panel()


func _text_width(label: Label) -> float:
	var font: Font = label.get_theme_font("font")
	var font_size: int = label.get_theme_font_size("font_size")
	var width: float = 0.0
	for line: String in label.text.split("\n"):
		width = maxf(width, font.get_string_size(line, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size).x)
	return ceilf(width)


func _fit_panel() -> void:
	_panel.size = Vector2(_card_width, 0.0)


func _position_panel() -> void:
	_panel.position = Vector2(maxf(EDGE_GAP, size.x - _panel.size.x - EDGE_GAP), EDGE_GAP)
