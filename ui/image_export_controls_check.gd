extends "res://ui/laboratory_controls_check.gd"

var _image_history: Dictionary = {}
var _last_image_export: String = ""
var _image_directory_edit: LineEdit


func _initialize() -> void:
	super._initialize()
	_deadline_usec = Time.get_ticks_usec() + 200000000


func _digest(bytes: PackedByteArray) -> String:
	var context: HashingContext = HashingContext.new()
	if context.start(HashingContext.HASH_SHA256) != OK or context.update(bytes) != OK:
		_fail("Cannot hash actual image bytes")
		return ""
	return context.finish().hex_encode()


func _dialog_record(dialog: FileDialog, prefix: String, result: Dictionary) -> Dictionary:
	var record: Dictionary = super._dialog_record(dialog, prefix, result)
	if not dialog.visible or prefix != "main:" or str(_application.get("_file_operation")) != "image":
		return record
	if _image_directory_edit == null:
		var directories: Array[LineEdit] = []
		for child: Node in dialog.get_vbox().find_children("*", "LineEdit", true, false):
			var edit: LineEdit = child as LineEdit
			if edit != dialog.get_line_edit() and edit.text == dialog.current_dir:
				directories.append(edit)
		if directories.size() != 1:
			_fail("Image chooser must expose one actual current directory entry")
			return record
		_image_directory_edit = directories[0]
	if not is_instance_valid(_image_directory_edit) or not dialog.is_ancestor_of(_image_directory_edit):
		_fail("Observed image directory entry left its actual chooser tree")
		return record
	_control_record(_image_directory_edit, prefix, result)
	record["Directory"] = prefix + str(_image_directory_edit.name)
	record["directory_text"] = _image_directory_edit.text
	var filters: Array[OptionButton] = []
	for child: Node in dialog.get_line_edit().get_parent().get_children():
		if child is OptionButton:
			filters.append(child as OptionButton)
	if filters.size() != 1:
		_fail("Image chooser must expose one actual filename filter")
		return record
	var filter: OptionButton = filters[0]
	_control_record(filter, prefix, result)
	var labels: Array[String] = []
	for index: int in filter.item_count:
		labels.append(filter.get_item_text(index))
	record["filter"] = {"control": prefix + str(filter.name), "selected": filter.selected, "labels": labels, "open": filter.get_popup().visible, "focused": filter.get_popup().get_focused_item()}
	var lists: Array[ItemList] = []
	for child: Node in dialog.get_vbox().find_children("*", "ItemList", true, false):
		var list: ItemList = child as ItemList
		if list.item_count > 0:
			var metadata: Variant = list.get_item_metadata(0)
			if typeof(metadata) == TYPE_DICTIONARY and metadata.has("dir") and metadata.has("name"):
				lists.append(list)
	if lists.size() != 1 or lists[0].item_count > 256:
		_fail("Image chooser must expose one bounded actual file list")
		return record
	var list: ItemList = lists[0]
	_control_record(list, prefix, result)
	var items: Array[String] = []
	for index: int in list.item_count:
		items.append(list.get_item_text(index))
	record["file_list"] = {"control": prefix + str(list.name), "items": items, "selected": list.get_selected_items()}
	return record


func _observation(state: Dictionary) -> Dictionary:
	var result: Dictionary = super._observation(state)
	result["save_path"] = str(_application.get("_save_path"))
	var pending: BgrImageExport = _application.get("_pending_image") as BgrImageExport
	result["image_pending"] = pending != null
	result["image_export_id"] = ""
	if pending != null:
		var identity: String = str(pending.get_instance_id())
		result["image_export_id"] = identity
		if not _image_history.has(identity):
			if _image_history.size() >= 12:
				_fail("Image journey exceeded its declared token observation bound")
				return result
			var snapshot: Dictionary = pending.snapshot()
			if not bool(snapshot.get("ok", false)):
				_fail("Actual pending export has no readable image")
				return result
			var rgb: PackedByteArray = snapshot["rgb"]
			var width: int = int(snapshot["width"])
			var height: int = int(snapshot["height"])
			if width <= 0 or height <= 0 or rgb.size() != width * height * 3:
				_fail("Actual pending export has inconsistent dimensions")
				return result
			_image_history[identity] = {"width": width, "height": height, "rgb": rgb.duplicate(), "rgb_sha256": _digest(rgb), "created_frame": Engine.get_process_frames(), "tick": int(state["tick"])}
		_last_image_export = identity
	result["last_image_export_id"] = _last_image_export
	result["image_exports"] = {}
	for identity: String in _image_history:
		var recorded: Dictionary = _image_history[identity]
		result["image_exports"][identity] = {"width": recorded["width"], "height": recorded["height"], "rgb_sha256": recorded["rgb_sha256"], "created_frame": recorded["created_frame"], "observed_frames": mini(4, Engine.get_process_frames() - int(recorded["created_frame"])), "tick": recorded["tick"]}
	var controls: Control = _application.get("_organism_controls") as Control
	var popup: PopupMenu = controls.get("_popup") as PopupMenu
	var actions: Dictionary = controls.get("_popup_actions")
	var focused: int = popup.get_focused_item()
	result["organism_menu"] = {"visible": popup.visible, "focused_action": str(actions.get(popup.get_item_id(focused), "")) if focused >= 0 else ""}
	return result


func _checkpoint(label: String) -> void:
	await process_frame
	await RenderingServer.frame_post_draw
	var request: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(_output_directory.path_join("checkpoint-request.json")))
	var checked: Dictionary = {}
	if request.has("image_path"):
		var path: String = str(request["image_path"]).simplify_path()
		var identity: String = str(request.get("export_id", ""))
		if not path.is_absolute_path() or path.get_base_dir() != _output_directory.simplify_path() or not _image_history.has(identity):
			_fail("Image readback must name an owned file and an actually observed token")
			return
		var png: PackedByteArray = FileAccess.get_file_as_bytes(path)
		if png.is_empty() or png.size() > 96 * 1024 * 1024:
			_fail("Actual saved PNG is absent or exceeds the declared byte bound")
			return
		var image: Image = Image.new()
		if image.load_png_from_buffer(png) != OK:
			_fail("Actual saved PNG cannot be decoded")
			return
		image.convert(Image.FORMAT_RGB8)
		var expected: Dictionary = _image_history[identity]
		if image.get_width() != expected["width"] or image.get_height() != expected["height"] or image.get_data() != expected["rgb"]:
			_fail("Saved PNG differs from the actual frozen token dimensions or RGB pixels")
			return
		checked = {"export_id": identity, "width": image.get_width(), "height": image.get_height(), "pixels": image.get_width() * image.get_height(), "rgb_sha256": expected["rgb_sha256"], "png_sha256": _digest(png), "path": path}
	var screenshot: String = _output_directory.path_join(label + ".png")
	if root.get_texture().get_image().save_png(screenshot) != OK or not _capture_windows(_application, label):
		_fail("Cannot preserve actual image journey pixels")
		return
	var observed: Dictionary = _observation(_application.get("_state"))
	observed["label"] = label
	observed["screenshot"] = screenshot
	observed["image_check"] = checked
	print("BGR_CRISP_CHECKPOINT " + JSON.stringify(observed, "", true, true))
	_capturing = false
