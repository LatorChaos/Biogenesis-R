extends "res://ui/laboratory_controls_check.gd"

var _alpha_pixels: Dictionary = {}


func _initialize() -> void:
	super._initialize()
	_deadline_usec = Time.get_ticks_usec() + 200000000


func _observation(state: Dictionary) -> Dictionary:
	var result: Dictionary = super._observation(state)
	var alphas: PackedByteArray = state["alphas"]
	result["packed_count"] = alphas.size()
	result["packed_alphas"] = Array(alphas) if alphas.size() <= 64 else []
	result["packed_colors"] = Array(state["colors"]) if alphas.size() <= 64 else []
	result["packed_segments"] = Array(state["segments"]) if alphas.size() <= 64 else []
	result["alpha_pixels"] = _alpha_pixels
	for laboratory: Dictionary in result["laboratories"]:
		for node: Node in _application.get_children():
			if str(node.get_instance_id()) != laboratory["id"]:
				continue
			laboratory["invalid"] = (node.get("_invalid") as Dictionary).duplicate()
			var input: LineEdit = node.find_child("GeneArgb", true, false) as LineEdit
			if input != null and not input.get_meta("retired", false):
				laboratory["argb_accessibility_name"] = input.accessibility_name
				laboratory["argb_accessibility_description"] = input.accessibility_description
				laboratory["argb_tooltip"] = input.tooltip_text
	return result


func _checkpoint(label: String) -> void:
	if label == "extra_alpha_render":
		await process_frame
		await RenderingServer.frame_post_draw
		var state: Dictionary = _application.get("_state")
		if not state["paused"] or int(state["tick"]) != 0 or int(state["population"]) != 1 or not state["selected"].is_empty():
			_fail("Alpha pixels require one unselected paused organism at tick zero")
			return
		var alphas: PackedByteArray = state["alphas"]
		var colors: PackedInt32Array = state["colors"]
		if alphas.is_empty() or alphas.size() != colors.size() or alphas.size() > 64:
			_fail("Actual alpha render arrays are absent or exceed the declared observation bound")
			return
		for index: int in alphas.size():
			if alphas[index] != 128 or colors[index] != 0xff0000:
				_fail("Actual pasted phenotype differs from the native half-red edit")
				return
		var canvas: Control = _application.get("_canvas") as Control
		var area: Rect2 = canvas.get_global_rect().grow(-3.0)
		var image: Image = root.get_texture().get_image()
		var count: int = 0
		var maximum: int = 0
		var first: Array[int] = []
		for y: int in range(ceili(area.position.y), floori(area.end.y)):
			for x: int in range(ceili(area.position.x), floori(area.end.x)):
				var color: Color = image.get_pixel(x, y)
				if color.r8 > 0 and color.r8 < 255 and color.g8 == 0 and color.b8 == 0:
					count += 1
					maximum = maxi(maximum, color.r8)
					if first.is_empty():
						first = [x, y, color.r8, color.g8, color.b8]
		if count == 0:
			_fail("The actual canvas has no visible nonopaque red organism pixel")
			return
		_alpha_pixels = {"count": count, "maximum_red": maximum, "first_xy_rgb": first, "canvas_rect": [area.position.x, area.position.y, area.size.x, area.size.y]}
	await super._checkpoint(label)
