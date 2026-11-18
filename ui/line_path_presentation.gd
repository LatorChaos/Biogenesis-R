extends Control

signal line_submitted

var _submitted: bool = false


func _init() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	focus_mode = Control.FOCUS_NONE


func _ready() -> void:
	await line_submitted
	await RenderingServer.frame_post_draw
	queue_free()


func _draw() -> void:
	if _submitted:
		return
	draw_multiline_colors(PackedVector2Array([Vector2(1.0, 1.0), Vector2(2.0, 1.0)]), PackedColorArray([Color(1.0, 1.0, 1.0, 0.0)]), 1.0, true)
	_submitted = true
	line_submitted.emit()
