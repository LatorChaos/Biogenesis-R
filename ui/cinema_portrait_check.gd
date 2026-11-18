extends SceneTree

const Cards = preload("res://ui/living_cards.gd")
const Preview = preload("res://ui/genome_preview.gd")


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
	var selection: Control = Preview.new()
	root.add_child(selection)
	if selection.get("fit_visible_content") != false or not selection.call("set_raster", image):
		push_error("Selected organism preview changed")
		quit(1)
		return
	if not preview.call("set_raster", {}) or preview.get("_portrait_source") != Rect2i():
		push_error("Cleared portrait retained old bounds")
		quit(1)
		return
	cards.free()
	selection.free()
	print("CINEMA_PORTRAIT_FIT_PASS")
	quit(0)
