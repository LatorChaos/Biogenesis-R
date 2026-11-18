extends "res://ui/player_controls_check.gd"

var _geometry_history: Dictionary = {}


func _stable_geometry(key: String, signature: Variant, valid: bool) -> Dictionary:
	var frame: int = Engine.get_process_frames()
	var previous: Dictionary = _geometry_history.get(key, {})
	var samples: int = 0
	if valid:
		samples = 1
		if previous.get("signature") == signature:
			if int(previous.get("frame", -1)) == frame:
				samples = int(previous.get("samples", 0))
			elif int(previous.get("frame", -1)) == frame - 1:
				samples = mini(3, int(previous.get("samples", 0)) + 1)
	_geometry_history[key] = {"signature": signature, "frame": frame, "samples": samples}
	return {"ready": samples >= 3, "samples": samples}


func _rect(values: Array) -> Rect2:
	return Rect2(float(values[0]), float(values[1]), float(values[2]), float(values[3]))


func _laboratory_layout(laboratory: Dictionary, observed: Dictionary) -> Dictionary:
	var prefix: String = laboratory["prefix"]
	if not laboratory["state"].has("genes") or not laboratory["state"].has("properties"):
		return _stable_geometry(prefix, {}, false)
	var window_rect: Rect2 = Rect2(Vector2(laboratory["position"][0], laboratory["position"][1]), Vector2(laboratory["size"][0], laboratory["size"][1]))
	var signature: Dictionary = {"window": laboratory["position"] + laboratory["size"], "parent_size": [root.size.x, root.size.y], "tab": laboratory["tab"], "genes": laboratory["state"]["genes"], "properties": laboratory["state"]["properties"], "error": laboratory["error"], "controls": {}, "identities": {}, "texts": {}}
	for key: String in observed["controls"]:
		if key.begins_with(prefix):
			signature["controls"][key] = observed["controls"][key]
			signature["identities"][key] = observed["control_details"].get(key, {}).get("identity", 0)
			if observed["texts"].has(key):
				signature["texts"][key] = observed["texts"][key]
	var required: Array[String] = ["CommitGenome", "CancelLaboratory", "ClearGenes", "ImportLaboratory", "tab:0", "tab:1"]
	if int(laboratory["tab"]) == 0:
		required.append_array(["AddGene", "InsertGene", "DeleteGene", "GeneList"])
	var rectangles: Dictionary = {}
	for name: String in required:
		if not observed["controls"].has(prefix + name):
			return _stable_geometry(prefix, signature, false)
		var area: Rect2 = _rect(observed["controls"][prefix + name])
		if area.size.x <= 0.0 or area.size.y <= 0.0 or not window_rect.encloses(area):
			return _stable_geometry(prefix, signature, false)
		rectangles[name] = area
	for first: int in required.size():
		for second: int in range(first + 1, required.size()):
			if rectangles[required[first]].intersects(rectangles[required[second]]):
				return _stable_geometry(prefix, signature, false)
	if int(laboratory["tab"]) == 0:
		if rectangles["AddGene"].position.y < rectangles["tab:0"].end.y or rectangles["InsertGene"].position.y < rectangles["AddGene"].end.y or rectangles["DeleteGene"].position.y < rectangles["InsertGene"].end.y:
			return _stable_geometry(prefix, signature, false)
	var fields: Array[String] = []
	if int(laboratory["tab"]) == 0 and not laboratory["state"]["genes"].is_empty():
		fields.assign(["GeneLength", "GeneAngle", "GeneBranch", "GeneColor"])
		if observed["controls"].has(prefix + "GeneColor") and rectangles["AddGene"].position.y < _rect(observed["controls"][prefix + "GeneColor"]).end.y:
			return _stable_geometry(prefix, signature, false)
	elif int(laboratory["tab"]) == 1:
		for name: String in ["symmetry", "mirror", "mutation_rate", "activity", "cream", "fallow", "spore", "adapt_spore", "plague", "disperse_children", "generation_battle", "sibling_battle", "altruist", "familial", "social", "peaceful", "passive", "clockwise", "mimic_all", "modifies_pink", "modifies_lilac", "modifies_sky", "selfish"]:
			fields.append("Genome_" + name)
	var previous_end: float = -INF
	for name: String in fields:
		if not observed["controls"].has(prefix + name):
			return _stable_geometry(prefix, signature, false)
		var area: Rect2 = _rect(observed["controls"][prefix + name])
		if area.size.x <= 0.0 or area.size.y <= 0.0 or area.position.y < previous_end:
			return _stable_geometry(prefix, signature, false)
		previous_end = area.end.y
	return _stable_geometry(prefix, signature, true)


func _initialize() -> void:
	super._initialize()
	_deadline_usec = Time.get_ticks_usec() + 390000000


func _window_origin(window: Window) -> Vector2:
	if window == root:
		return Vector2.ZERO
	return Vector2(window.position) if root.gui_embed_subwindows else Vector2(window.position - root.position)


func _controls_inside_viewport() -> bool:
	for node: Node in _application.get_children():
		if node is Window and node.visible and node.get_script() == load("res://ui/laboratory_window.gd"):
			var decoration: Dictionary = _laboratory_decorations(node as Window)
			if not decoration["supported"] or not decoration["inside"]:
				_fail("Laboratory decoration exceeds its parent viewport: " + str(decoration))
				return false
	for node: Node in _application.find_children("*", "Control", true, false):
		var control: Control = node as Control
		if not control.is_visible_in_tree():
			continue
		var window: Window = control.get_window()
		var visible_rect: Rect2 = control.get_global_rect()
		var parent: Node = control.get_parent()
		while parent != null and parent != window:
			if parent is Control and (parent as Control).clip_contents:
				visible_rect = visible_rect.intersection((parent as Control).get_global_rect())
			parent = parent.get_parent()
		if visible_rect.size.x <= 0.0 or visible_rect.size.y <= 0.0:
			continue
		var bounds: Rect2 = Rect2(Vector2.ZERO, Vector2(window.size)).grow(0.1)
		if not bounds.encloses(visible_rect):
			_fail("Visible control exceeds its Window: " + str(control.name))
			return false
	return true


func _laboratory_decorations(laboratory: Window) -> Dictionary:
	var client: Rect2 = Rect2(_window_origin(laboratory), Vector2(laboratory.size))
	if not laboratory.is_embedded() or laboratory.borderless:
		return {"supported": true, "inside": true, "painted_rect": [client.position.x, client.position.y, client.size.x, client.size.y], "close_rect": []}
	var painted: Rect2 = client
	var supported: bool = true
	var styles: Dictionary = {}
	for style_name: String in ["embedded_border", "embedded_unfocused_border"]:
		var border: StyleBoxFlat = laboratory.get_theme_stylebox(style_name) as StyleBoxFlat
		if border == null:
			supported = false
			continue
		var margins: Array[float] = [border.get_expand_margin(SIDE_LEFT), border.get_expand_margin(SIDE_TOP), border.get_expand_margin(SIDE_RIGHT), border.get_expand_margin(SIDE_BOTTOM)]
		var area: Rect2 = client.grow_individual(margins[0], margins[1], margins[2], margins[3])
		if border.shadow_size > 0:
			var shadow: Rect2 = area.grow(float(border.shadow_size))
			shadow.position += border.shadow_offset
			area = area.merge(shadow)
		painted = painted.merge(area)
		styles[style_name] = {"expand": margins, "shadow_size": border.shadow_size, "shadow_offset": [border.shadow_offset.x, border.shadow_offset.y]}
	var close_position: Vector2 = client.position + Vector2(float(laboratory.size.x - laboratory.get_theme_constant("close_h_offset")), -float(laboratory.get_theme_constant("close_v_offset")))
	var close: Rect2 = Rect2(close_position, laboratory.get_theme_icon("close").get_size()).merge(Rect2(close_position, laboratory.get_theme_icon("close_pressed").get_size()))
	var bounds: Rect2 = Rect2(Vector2.ZERO, Vector2(root.size))
	return {"supported": supported, "inside": bounds.encloses(painted) and bounds.encloses(close), "painted_rect": [painted.position.x, painted.position.y, painted.size.x, painted.size.y], "close_rect": [close.position.x, close.position.y, close.size.x, close.size.y], "styles": styles}


func _control_record(control: Control, prefix: String, result: Dictionary) -> void:
	if not control.is_visible_in_tree() or bool(control.get_meta("retired", false)):
		return
	var window: Window = control.get_window()
	var origin: Vector2 = _window_origin(window)
	var rect: Rect2 = control.get_global_rect()
	rect.position += origin
	var key: String = prefix + str(control.name)
	result["controls"][key] = [rect.position.x, rect.position.y, rect.size.x, rect.size.y]
	var clip: Rect2 = Rect2(origin, Vector2(window.size))
	var parent: Node = control.get_parent()
	var scroll_key: String = ""
	while parent != null and parent != window:
		if parent is ScrollContainer:
			var scroll: ScrollContainer = parent as ScrollContainer
			var area: Rect2 = scroll.get_global_rect()
			area.position += origin
			clip = clip.intersection(area)
			scroll_key = prefix + "scroll_" + str(scroll.get_instance_id())
			result["controls"][scroll_key] = [area.position.x, area.position.y, area.size.x, area.size.y]
		parent = parent.get_parent()
	result["control_details"][key] = {"inside": clip.encloses(rect), "scroll": scroll_key, "clip": [clip.position.x, clip.position.y, clip.size.x, clip.size.y], "identity": control.get_instance_id(), "window_rect": [origin.x, origin.y, window.size.x, window.size.y]}
	if control is LineEdit:
		result["texts"][key] = (control as LineEdit).text
	elif control is CheckBox:
		result["toggles"][key] = (control as CheckBox).button_pressed
	elif control is OptionButton:
		var palette: OptionButton = control as OptionButton
		result["palettes"][key] = {"selected": palette.selected, "count": palette.item_count, "open": palette.get_popup().visible, "focused_index": palette.get_popup().get_focused_item()}


func _collect_controls(node: Node, prefix: String, result: Dictionary) -> void:
	for child: Node in node.get_children():
		if child is Window:
			continue
		if child is LineEdit or child is Button or child is ItemList:
			_control_record(child as Control, prefix, result)
		_collect_controls(child, prefix, result)


func _dialog_record(dialog: FileDialog, prefix: String, result: Dictionary) -> Dictionary:
	var record: Dictionary = {"visible": dialog.visible, "path": dialog.current_path, "directory": dialog.current_dir, "file": dialog.current_file}
	if not dialog.visible:
		return record
	var line: LineEdit = dialog.get_line_edit()
	var okay: Button = dialog.get_ok_button()
	var cancel: Button = dialog.get_cancel_button()
	for entry: Array in [["FileName", line], ["FileOkay", okay], ["FileCancel", cancel]]:
		var control: Control = entry[1] as Control
		var rect: Rect2 = control.get_global_rect()
		rect.position += _window_origin(dialog)
		result["controls"][prefix + str(entry[0])] = [rect.position.x, rect.position.y, rect.size.x, rect.size.y]
		var origin: Vector2 = _window_origin(dialog)
		result["control_details"][prefix + str(entry[0])] = {"inside": true, "scroll": "", "clip": [origin.x, origin.y, dialog.size.x, dialog.size.y], "identity": control.get_instance_id(), "window_rect": [origin.x, origin.y, dialog.size.x, dialog.size.y]}
		record[str(entry[0])] = prefix + str(entry[0])
	record["filename_text"] = line.text
	result["texts"][prefix + "FileName"] = line.text
	return record


func _observation(state: Dictionary) -> Dictionary:
	var result: Dictionary = super._observation(state)
	result["control_details"] = {}
	result["toggles"] = {}
	result["palettes"] = {}
	result["laboratories"] = []
	result["clipboard_ready"] = bool(state.get("clipboard_ready", false))
	result["world_size"] = [float(state["width"]), float(state["height"])]
	result["root_has_focus"] = root.has_focus()
	var root_mouse: Vector2 = root.get_mouse_position()
	result["root_mouse"] = [root_mouse.x, root_mouse.y]
	var canvas: Control = _application.get("_canvas") as Control
	var canvas_mouse: Vector2 = canvas.get_local_mouse_position()
	result["canvas_mouse"] = [canvas_mouse.x, canvas_mouse.y]
	result["canvas_drag_button"] = int(canvas.get("_drag_button"))
	result["canvas_dragging"] = bool(canvas.get("_dragging"))
	var last_pick: Vector2 = _application.get("_placement_position")
	result["last_world_pick"] = [last_pick.x, last_pick.y]
	result["placement_mode"] = str(_application.get("_placement_mode"))
	result["file_operation"] = str(_application.get("_file_operation"))
	var genetics: MenuButton = _application.find_child("GeneticsMenu", true, false) as MenuButton
	_control_record(genetics, "", result)
	result["genetics_menu"] = genetics.get_popup().visible
	var focused: int = genetics.get_popup().get_focused_item()
	result["genetics_focused_id"] = genetics.get_popup().get_item_id(focused) if focused >= 0 else -1
	var world_context: PopupMenu = _application.get("_world_context") as PopupMenu
	var context_focus: int = world_context.get_focused_item()
	var context_origin: Vector2 = _window_origin(world_context)
	result["world_context"] = {"visible": world_context.visible, "focused_id": world_context.get_item_id(context_focus) if context_focus >= 0 else -1, "rect": [context_origin.x, context_origin.y, world_context.size.x, world_context.size.y]}
	var error: Label = _application.get("_error_label") as Label
	result["application_error"] = error.text if error.visible else ""
	var main_dialog: FileDialog = _application.get("_file_dialog") as FileDialog
	result["file_dialog"] = _dialog_record(main_dialog, "main:", result)
	var overwrite: ConfirmationDialog = _application.get("_overwrite_dialog") as ConfirmationDialog
	result["overwrite"] = overwrite.visible
	if overwrite.visible:
		for entry: Array in [["OverwriteOkay", overwrite.get_ok_button()], ["OverwriteCancel", overwrite.get_cancel_button()]]:
			var control: Control = entry[1] as Control
			var rect: Rect2 = control.get_global_rect()
			rect.position += _window_origin(overwrite)
			result["controls"][str(entry[0])] = [rect.position.x, rect.position.y, rect.size.x, rect.size.y]
			var origin: Vector2 = _window_origin(overwrite)
			result["control_details"][str(entry[0])] = {"inside": true, "scroll": "", "clip": [origin.x, origin.y, overwrite.size.x, overwrite.size.y], "identity": control.get_instance_id(), "window_rect": [origin.x, origin.y, overwrite.size.x, overwrite.size.y]}
	var inspector: Control = _application.find_child("SelectedGenomePreview", true, false) as Control
	if inspector.is_visible_in_tree():
		_control_record(inspector, "", result)
		result["preview_popup"] = bool(inspector.call("has_open_popup"))
		var preview_popup: PopupMenu = inspector.get("_popup") as PopupMenu
		var preview_focus: int = preview_popup.get_focused_item()
		result["preview_focused_id"] = preview_popup.get_item_id(preview_focus) if preview_focus >= 0 else -1
	else:
		result["preview_popup"] = false
		result["preview_focused_id"] = -1
	for node: Node in _application.get_children():
		if not node is Window or str(node.name).begins_with("Genotype") or node.get_script() != load("res://ui/laboratory_window.gd"):
			continue
		var laboratory: Window = node as Window
		if laboratory.is_queued_for_deletion() or bool(laboratory.get("_closed")):
			continue
		var identity: String = str(laboratory.get_instance_id())
		var prefix: String = "lab:" + identity + ":"
		var draft_state: Dictionary = laboratory.get("_state")
		var origin: Vector2 = _window_origin(laboratory)
		var tabs: TabContainer = laboratory.find_child("LaboratoryTabs", true, false) as TabContainer
		var bar: TabBar = tabs.get_tab_bar()
		for index: int in tabs.get_tab_count():
			var tab_rect: Rect2 = bar.get_tab_rect(index)
			tab_rect.position += bar.global_position + origin
			result["controls"][prefix + "tab:" + str(index)] = [tab_rect.position.x, tab_rect.position.y, tab_rect.size.x, tab_rect.size.y]
		_collect_controls(laboratory, prefix, result)
		var local_error: Label = laboratory.get("_error") as Label
		var import_dialog: FileDialog = laboratory.get("_import_dialog") as FileDialog
		var focus: Control = laboratory.gui_get_focus_owner()
		result["laboratories"].append({"id": identity, "prefix": prefix, "position": [origin.x, origin.y], "size": [laboratory.size.x, laboratory.size.y], "tab": tabs.current_tab, "focus": "" if focus == null else str(focus.name), "has_focus": laboratory.has_focus(), "embedded": laboratory.is_embedded(), "exclusive": laboratory.exclusive, "popup": laboratory.popup_window, "transient": laboratory.transient, "selected_gene": int(laboratory.get("_selected_gene")), "state": draft_state, "error": local_error.text, "import_dialog": _dialog_record(import_dialog, prefix, result), "decoration": _laboratory_decorations(laboratory)})
	var dialog_groups: Array = [["main:FileName", "main:FileOkay", "main:FileCancel"], ["OverwriteOkay", "OverwriteCancel"]]
	for laboratory: Dictionary in result["laboratories"]:
		dialog_groups.append([laboratory["prefix"] + "FileName", laboratory["prefix"] + "FileOkay", laboratory["prefix"] + "FileCancel"])
	for group: Array in dialog_groups:
		var coherent: bool = true
		for first: int in group.size():
			for second: int in range(first + 1, group.size()):
				if result["controls"].has(group[first]) and result["controls"].has(group[second]) and _rect(result["controls"][group[first]]).intersects(_rect(result["controls"][group[second]])):
					coherent = false
		for key: String in group:
			if result["control_details"].has(key):
				result["control_details"][key]["coherent"] = coherent
	result["control_ready"] = {}
	for key: String in result["controls"]:
		var area: Rect2 = _rect(result["controls"][key])
		var detail: Dictionary = result["control_details"].get(key, {})
		var bounds: Rect2 = _rect(detail.get("window_rect", [0, 0, root.size.x, root.size.y]))
		var valid: bool = area.size.x > 0.0 and area.size.y > 0.0 and bounds.encloses(area) and bool(detail.get("inside", true)) and bool(detail.get("coherent", true))
		var sample: Dictionary = {"rect": result["controls"][key], "identity": detail.get("identity", 0), "window": [bounds.position.x, bounds.position.y, bounds.size.x, bounds.size.y]}
		result["control_ready"][key] = _stable_geometry("control:" + key, sample, valid)["ready"]
	for laboratory: Dictionary in result["laboratories"]:
		var readiness: Dictionary = _laboratory_layout(laboratory, result)
		laboratory["layout_ready"] = readiness["ready"]
		laboratory["layout_stable_frames"] = readiness["samples"]
	var projection: Dictionary = {"canvas": result["controls"]["WorldCanvas"], "zoom": result["zoom"], "offset": result["offset"]}
	var valid_projection: bool = float(projection["zoom"]) > 0.0 and is_finite(float(projection["zoom"]))
	for value: float in projection["canvas"] + projection["offset"]:
		valid_projection = valid_projection and is_finite(value)
	valid_projection = valid_projection and float(projection["canvas"][2]) > 0.0 and float(projection["canvas"][3]) > 0.0
	var projection_readiness: Dictionary = _stable_geometry("world_projection", {"projection": projection, "placement_mode": result["placement_mode"], "parent_size": [root.size.x, root.size.y]}, valid_projection)
	result["world_projection"] = projection
	result["world_projection_ready"] = projection_readiness["ready"]
	result["world_projection_stable_frames"] = projection_readiness["samples"]
	return result


func _capture_windows(parent: Node, label: String) -> bool:
	for node: Node in parent.get_children():
		if node is Window and node.visible:
			var window: Window = node as Window
			var image: Image = window.get_texture().get_image()
			if image.save_png(_output_directory.path_join(label + "_window_" + str(window.get_instance_id()) + ".png")) != OK:
				_fail("Cannot preserve laboratory or dialog pixels: " + label)
				return false
		if not _capture_windows(node, label):
			return false
	return true


func _checkpoint(label: String) -> void:
	await process_frame
	await RenderingServer.frame_post_draw
	if not _capture_windows(_application, label):
		return
	if label == "lab_raw_point":
		var checked: int = 0
		for node: Node in _application.get_children():
			if not node is Window or node.get_script() != load("res://ui/laboratory_window.gd") or not node.visible:
				continue
			var laboratory: Window = node as Window
			var preview_control: Control = laboratory.find_child("LaboratoryPreview", true, false) as Control
			var displayed: Dictionary = preview_control.get("_preview")
			var points: PackedInt32Array = displayed["points"]
			if points.is_empty():
				_fail("Raw point preview has no actual endpoints")
				return
			for value: int in points:
				if value != 0:
					_fail("Raw point preview endpoint differs from the owned zero-length input")
					return
			var translation: PackedInt32Array = displayed["translation"]
			var source_point: Vector2i = Vector2i(preview_control.global_position) + Vector2i(translation[0], translation[1])
			var image: Image = laboratory.get_texture().get_image()
			if source_point.x < 0 or source_point.y < 0 or source_point.x >= image.get_width() or source_point.y >= image.get_height():
				_fail("Actual preview translation leaves its window image")
				return
			var actual: Color = image.get_pixelv(source_point)
			if actual.to_rgba32() != Color.GREEN.to_rgba32():
				_fail("Raw point pixel differs at %s: expected GREEN, actual %s" % [source_point, actual])
				return
			checked += 1
		if checked != 1:
			_fail("Raw point checkpoint requires exactly one actual laboratory")
			return
	await super._checkpoint(label)
