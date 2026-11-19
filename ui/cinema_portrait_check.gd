extends SceneTree

const Cards = preload("res://ui/living_cards.gd")
const Preview = preload("res://ui/genome_preview.gd")
const OrganismControls = preload("res://ui/organism_controls.gd")


func _initialize() -> void:
	_run.call_deferred()


func _run() -> void:
	var cards: Control = Cards.new()
	root.add_child(cards)
	await process_frame
	var preview: Control = cards.get("_preview")
	if preview.get("fit_visible_content") != true:
		push_error("Cinema portraits do not fit their visible organism")
		quit(1)
		return
	for leader: Control in cards.get("_leader_previews"):
		if leader.get("fit_visible_content") != true:
			push_error("A leaderboard portrait does not fit its organism")
			quit(1)
			return
	var pixels: PackedByteArray = PackedByteArray()
	pixels.resize(30000)
	pixels[(45 * 100 + 40) * 3] = 255
	pixels[(54 * 100 + 44) * 3 + 1] = 255
	var image: Dictionary = {"width": 100, "height": 100, "rgb": pixels}
	if not preview.call("set_raster", image) or preview.get("_portrait_source") != Rect2i(38, 43, 9, 14):
		push_error("Sparse organism bounds are not fitted with padding")
		quit(1)
		return
	if preview.call("_fit_target").size != Vector2(54, 84):
		push_error("Record portrait scale changed")
		quit(1)
		return
	pixels.fill(0)
	pixels[0] = 255
	pixels[29999] = 255
	if not preview.call("set_raster", image) or preview.get("_portrait_source") != Rect2i(0, 0, 100, 100):
		push_error("Portrait fitting clipped edge pixels")
		quit(1)
		return
	pixels.fill(0)
	if not preview.call("set_raster", image) or preview.get("_portrait_source") != Rect2i(0, 0, 100, 100):
		push_error("An empty portrait did not retain its full canvas")
		quit(1)
		return
	var controls: Control = OrganismControls.new()
	root.add_child(controls)
	controls.show()
	await process_frame
	await process_frame
	var selection: Control = controls.get("_preview")
	if selection.size != Vector2(100, 100) or selection.get_parent().size.y <= 100.0:
		push_error("Selected preview stretches with the taller details row")
		quit(1)
		return
	pixels[0] = 255
	if not selection.get("fit_visible_content") or not selection.call("set_raster", image) or selection.get("_portrait_source") != Rect2i(0, 0, 3, 3):
		push_error("Selected organism does not fit its visible bounds")
		quit(1)
		return
	selection.size = Vector2(100, 100)
	if selection.call("_fit_target") != Rect2(2, 2, 96, 96) or not selection.get("copy_enabled"):
		push_error("Selected organism does not fill its area safely or retain its copy menu")
		quit(1)
		return
	pixels[29999] = 255
	selection.call("set_raster", image)
	selection.custom_minimum_size = Vector2.ZERO
	selection.size = Vector2(120, 80)
	if selection.call("_fit_target") != Rect2(22, 2, 76, 76):
		push_error("Large organism fit does not preserve aspect ratio and edge margin: %s %s" % [selection.call("_fit_target"), selection.size])
		quit(1)
		return
	var texture: ImageTexture = selection.get("_raster")
	if not selection.call("set_raster", image) or selection.get("_raster") != texture:
		push_error("Repeated raster did not retain its texture")
		quit(1)
		return
	pixels.fill(0)
	pixels[(20 * 100 + 10) * 3] = 255
	selection.call("set_raster", image)
	if selection.get("_portrait_source") != Rect2i(8, 18, 5, 5):
		push_error("Changed raster retained stale bounds")
		quit(1)
		return
	var reshaped: Dictionary = {"width": 50, "height": 200, "rgb": pixels}
	if not selection.call("set_raster", reshaped, Vector2i(50, 200)) or selection.get("_portrait_source") != Rect2i(8, 38, 5, 5):
		push_error("Changed dimensions retained stale bounds")
		quit(1)
		return
	selection.set("fit_visible_content", false)
	selection.call("set_raster", reshaped, Vector2i(50, 200))
	if selection.get("_portrait_source") != Rect2i(0, 0, 50, 200):
		push_error("Changed fit mode retained stale bounds")
		quit(1)
		return
	selection.set("fit_visible_content", true)
	selection.call("set_raster", {})
	if selection.get("_raster") != null or not selection.call("set_raster", reshaped, Vector2i(50, 200)) or selection.get("_portrait_source") != Rect2i(8, 38, 5, 5):
		push_error("Clearing a raster retained stale cache state")
		quit(1)
		return
	if not preview.call("set_raster", {}) or preview.get("_portrait_source") != Rect2i():
		push_error("Cleared portrait retained old bounds")
		quit(1)
		return
	cards.free()
	controls.free()
	print("CINEMA_PORTRAIT_FIT_PASS")
	quit(0)
