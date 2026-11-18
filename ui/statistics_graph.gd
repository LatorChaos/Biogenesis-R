# Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
# SPDX-License-Identifier: GPL-2.0-or-later

extends Control

var _series: Array[Dictionary] = []
var _names: PackedStringArray = PackedStringArray()
var _legend: Array[Label] = []


func configure(series: Array[Dictionary], names: PackedStringArray) -> void:
	assert(series.size() == 3 and names.size() == 3)
	_series = series.duplicate(true)
	_names = names.duplicate()
	custom_minimum_size = Vector2(300, 105)
	clip_contents = true
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	if is_node_ready():
		_rebuild_legend()
	queue_redraw()


func _ready() -> void:
	_rebuild_legend()


func _rebuild_legend() -> void:
	for node: Node in get_children():
		remove_child(node)
		node.queue_free()
	_legend.clear()
	for index: int in range(3):
		var label: Label = Label.new()
		label.name = "Legend%d" % index
		label.text = _names[index]
		label.position = Vector2(112, index * 24)
		label.add_theme_color_override("font_color", _argb(int(_series[index]["argb"])))
		label.mouse_filter = Control.MOUSE_FILTER_IGNORE
		add_child(label)
		_legend.append(label)
	for index: int in range(2):
		var label: Label = Label.new()
		label.name = "Axis%d" % index
		label.text = "0" if index == 0 else "100"
		label.position = Vector2(0 if index == 0 else 76, 80)
		label.mouse_filter = Control.MOUSE_FILTER_IGNORE
		add_child(label)


func _draw() -> void:
	draw_rect(Rect2(Vector2.ZERO, size), Color.BLACK)
	for series: Dictionary in _series:
		var coordinates: PackedInt32Array = series["points"]
		if coordinates.size() < 4:
			continue
		var points: PackedVector2Array = PackedVector2Array()
		points.resize(coordinates.size() >> 1)
		for index: int in points.size():
			points[index] = Vector2(coordinates[index * 2], coordinates[index * 2 + 1])
		draw_polyline(points, _argb(int(series["argb"])), 1.0, false)


static func _argb(argb: int) -> Color:
	return Color8((argb >> 16) & 255, (argb >> 8) & 255, argb & 255, (argb >> 24) & 255)
