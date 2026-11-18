extends SceneTree

const Application = preload("res://ui/main.gd")
const OrganismControls = preload("res://ui/organism_controls.gd")


func _initialize() -> void:
	call_deferred("_run")


func _require(condition: bool, message: String) -> bool:
	if not condition:
		push_error(message)
		quit(1)
	return condition


func _run() -> void:
	var application: Control = Application.new()
	var controls: VBoxContainer = OrganismControls.new()
	root.add_child(controls)
	await process_frame
	var selected: Dictionary = {
		"id": 7, "genotype_available": false, "age": 1, "age_ticks": 256,
		"maximum_age": 80, "children": 2, "kills": 3, "infections": 4,
		"reproduce_energy": 50, "energy": 12.0, "mass": 8.0,
		"alive": true, "infected": false, "bounds": PackedInt32Array([10, 20, 30, 40]),
		"display": {"id": "7", "age": "1", "maximum_age": "80", "children": "2", "kills": "3", "infections": "4", "reproduce_energy": "50", "energy": "12", "mass": "8"}
	}
	var response: Dictionary = {
		"selected": selected, "selected_is_tracked": true, "tracking": true,
		"tracking_center": PackedFloat64Array([25.0, 40.0]),
		"tracking_path": PackedInt32Array(), "max_velocity": 2.0, "methane": 0.0,
		"genotype_preview": {}, "genotype_preview_error": "Primary genotype unavailable"
	}
	if not _require(bool(application.call("_valid_selection", response)), "Nullable selected actor was rejected"):
		return
	var pixels: PackedByteArray = PackedByteArray()
	pixels.resize(30000)
	pixels[0] = 255
	var image: Dictionary = {"width": 100, "height": 100, "rgb": pixels}
	if not _require(bool(controls.call("set_genome_preview", image, "")), "Valid selected RGB8 preview was rejected"):
		return
	var preview: Control = controls.get("_preview") as Control
	var texture: ImageTexture = preview.get("_raster") as ImageTexture
	if not _require(texture != null and bool(controls.call("set_genome_preview", image, "")) and preview.get("_raster") == texture, "Repeated selected preview replaced its texture"):
		return
	for invalid: Dictionary in [
		{"width": true, "height": 100, "rgb": pixels},
		{"width": 99, "height": 100, "rgb": pixels},
		{"width": 100, "height": 100.0, "rgb": pixels},
		{"width": 100, "height": 100, "rgb": PackedByteArray([1])},
		{"width": 100, "height": 100, "rgb": []},
		{"width": 100, "height": 100}
	]:
		var malformed: Dictionary = response.duplicate(true)
		malformed["genotype_preview"] = invalid
		if not _require(not bool(application.call("_valid_selection", malformed)), "Malformed selected RGB8 packet was accepted"):
			return
		if not _require(not bool(controls.call("set_genome_preview", invalid, "invalid")) and preview.get("_raster") == texture and texture.get_image().get_data() == pixels, "Invalid selected preview replaced the prior image"):
			return
	for field: String in ["genotype_preview", "genotype_preview_error"]:
		var missing: Dictionary = response.duplicate(true)
		missing.erase(field)
		if not _require(not bool(application.call("_valid_selection", missing)), "Missing selected preview field was accepted"):
			return
	if not _require(bool(controls.call("set_genome_preview", {}, "Primary genotype unavailable")) and preview.get("_raster") == null, "Empty selected preview retained pixels"):
		return
	controls.call("set_selection", selected, true, true)
	var summary: Label = controls.get("_summary") as Label
	var lineage: Label = controls.get("_lineage") as Label
	if not _require(controls.visible and "ID: 7" in summary.text and "Generation: Unavailable" in summary.text, "Selection identity or absent genotype was lost"):
		return
	if not _require("Mutation \u2031: Unavailable" in lineage.text and "Clade:  \u21b4 Unavailable" in lineage.text, "Absent genotype gained fabricated values"):
		return
	controls.call("show_context", Vector2(10.0, 10.0))
	var popup: PopupMenu = controls.get("_popup") as PopupMenu
	var actions: Dictionary = controls.get("_popup_actions")
	for index: int in range(popup.item_count):
		var id: int = popup.get_item_id(index)
		if actions.has(id):
			var action: String = str(actions[id])
			if not _require(popup.is_item_disabled(index) == (action == "export"), "Nullable selection changed unrelated action eligibility: " + action):
				return
	popup.hide()
	selected["generation"] = 99
	if not _require(not bool(application.call("_valid_selection", response)), "Absent genotype accepted a fabricated generation"):
		return
	selected.erase("generation")
	selected["genotype_available"] = true
	if not _require(not bool(application.call("_valid_selection", response)), "Available genotype accepted missing fields"):
		return
	selected["generation"] = 9
	selected["mutation_rate"] = 31
	selected["clade"] = "test-clade"
	selected["display"].merge({"generation": "9", "mutation_rate": "31", "clade": "test-clade"})
	if not _require(bool(application.call("_valid_selection", response)), "Existing nonnull selection contract failed"):
		return
	controls.call("set_selection", selected, true, true)
	if not _require("Generation: 9" in summary.text and "Mutation \u2031: 31" in lineage.text and "test-clade" in lineage.text, "Nonnull genetic labels changed"):
		return
	controls.free()
	application.free()
	print("NULLABLE_SELECTION_UI_PASS")
	quit(0)
