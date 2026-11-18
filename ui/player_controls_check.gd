extends "res://ui/crisp_check.gd"


func _observation(state: Dictionary) -> Dictionary:
	var result: Dictionary = super._observation(state)
	var selected: Dictionary = state["selected"].duplicate(true)
	if not selected.is_empty():
		selected["bounds"] = Array(selected["bounds"])
	result["selected"] = selected
	result["tracking"] = state["tracking"]
	result["selected_is_tracked"] = state["selected_is_tracked"]
	result["tracking_center"] = Array(state["tracking_center"])
	result["tracking_path"] = Array(state["tracking_path"])
	for field: String in ["oxygen", "carbon_dioxide", "methane"]:
		result[field] = state[field]
	for node_name: String in ["FeedOrganismButton", "WeakenOrganismButton", "ReproduceOrganismButton", "RejuvenateOrganismButton", "KillOrganismButton", "ReviveOrganismButton", "DisperseOrganismButton", "TrackOrganismButton", "AbortOrganismTrackingButton", "WorldActionsMenu", "GlobalActionsMenu"]:
		var control: Control = _application.find_child(node_name, true, false) as Control
		if control.is_visible_in_tree():
			var rect: Rect2 = control.get_global_rect()
			result["controls"][node_name] = [rect.position.x, rect.position.y, rect.size.x, rect.size.y]
	var controls: Control = _application.get("_organism_controls") as Control
	result["context_menu"] = controls.call("has_open_popup")
	result["world_context"] = (_application.get("_world_context") as PopupMenu).visible
	var menu: MenuButton = _application.get("_world_menu") as MenuButton
	result["world_menu"] = menu.get_popup().visible
	var focused_item: int = menu.get_popup().get_focused_item()
	result["world_menu_focused_id"] = menu.get_popup().get_item_id(focused_item) if focused_item >= 0 else -1
	var actions_popup: PopupMenu = (_application.get("_actions_menu") as MenuButton).get_popup()
	result["global_actions_menu"] = actions_popup.visible
	var actions_focused: int = actions_popup.get_focused_item()
	result["global_actions_focused_id"] = actions_popup.get_item_id(actions_focused) if actions_focused >= 0 else -1
	result["global_actions_item_count"] = actions_popup.item_count
	var candidates: Array[Dictionary] = []
	var extents: Dictionary[int, Rect2] = {}
	var segments: PackedFloat64Array = state["segments"]
	var identities: PackedInt64Array = state["organism_ids"]
	for index: int in range(identities.size()):
		var identity: int = identities[index]
		var start: Vector2 = Vector2(segments[index * 4], segments[index * 4 + 1])
		var end: Vector2 = Vector2(segments[index * 4 + 2], segments[index * 4 + 3])
		var bounds: Rect2 = extents.get(identity, Rect2(start, Vector2.ZERO))
		extents[identity] = bounds.expand(start).expand(end)
	for identity: int in extents:
		var bounds: Rect2 = extents[identity]
		var center: Vector2 = bounds.get_center()
		candidates.append({"id": identity, "position": [center.x, center.y], "extent": [bounds.size.x, bounds.size.y]})
		if candidates.size() == 128:
			break
	result["organism_points"] = candidates
	return result


func _checkpoint(label: String) -> void:
	await process_frame
	await RenderingServer.frame_post_draw
	var state: Dictionary = _application.get("_state")
	var selected: Dictionary = state["selected"]
	if not selected.is_empty():
		var canvas: Control = _application.get("_canvas") as Control
		var bounds: PackedInt32Array = selected["bounds"]
		var offset: Vector2 = canvas.get("_offset")
		var zoom: float = canvas.get("_zoom")
		var corner: Vector2 = canvas.global_position + offset + Vector2(bounds[0], bounds[1]) * zoom
		var center: Vector2 = canvas.global_position + offset + Vector2(bounds[0] + bounds[2] * 0.5, bounds[1] + bounds[3] * 0.5) * zoom
		if label == "player_minimum_selected" and not canvas.get_global_rect().has_point(center):
			_fail("The minimum inspector hides its selected organism")
			return
		if canvas.get_global_rect().grow(-3.0).has_point(corner):
			var image: Image = root.get_texture().get_image()
			var expected: int = (Color.WHITE if bool(selected["infected"]) else Color("ffc800")).to_rgba32()
			var found: bool = false
			for x: int in range(int(corner.x) - 2, int(corner.x) + 3):
				for y: int in range(int(corner.y) - 2, int(corner.y) + 3):
					if image.get_pixel(x, y).to_rgba32() == expected:
						found = true
			if not found:
				_fail("Selected source rectangle corner is not visible: " + label)
				return
	var controls: Control = _application.get("_organism_controls") as Control
	var popup: PopupMenu = controls.get("_popup") as PopupMenu
	var world_context: PopupMenu = _application.get("_world_context") as PopupMenu
	if world_context.visible:
		popup = world_context
	if popup.visible:
		var image: Image = popup.get_texture().get_image()
		if image.save_png(_output_directory.path_join(label + "_popup.png")) != OK:
			_fail("Cannot preserve actual context popup pixels")
			return
	await super._checkpoint(label)
