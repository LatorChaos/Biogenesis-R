extends SceneTree

const StatisticsWindow = preload("res://ui/statistics_window.gd")
var _checks: int = 0
var _failures: Array[String] = []
var _copies: int = 0
var _window: StatisticsWindow
var _simulation: BgrSimulation
var _mouse_notified: bool = false


func _initialize() -> void:
	_run.call_deferred()


func _check(ok: bool, message: String) -> void:
	_checks += 1
	if not ok:
		_failures.append(message)


func _clipboard() -> Dictionary:
	var opened: Dictionary = _simulation.open_laboratory("clipboard")
	if not bool(opened.get("ok", false)):
		return {}
	var packet: Dictionary = _simulation.laboratory_snapshot(opened["draft"], 32, 32)
	return {"genes": packet.get("genes"), "properties": packet.get("properties")}


func _pointer(control: Control, button_index: MouseButton = MOUSE_BUTTON_LEFT) -> void:
	var scroll: ScrollContainer = _window.find_child("StatisticsScroll", true, false) as ScrollContainer
	scroll.ensure_control_visible(control)
	await process_frame
	await process_frame
	var point: Vector2 = control.get_viewport().get_screen_transform() * control.get_global_transform_with_canvas() * (control.size * 0.5)
	if not _mouse_notified:
		root.notify_mouse_entered()
		_mouse_notified = true
	var motion: InputEventMouseMotion = InputEventMouseMotion.new()
	motion.window_id = root.get_window_id()
	motion.position = point
	motion.global_position = point
	motion.relative = Vector2.ONE
	Input.parse_input_event(motion)
	Input.flush_buffered_events()
	await process_frame
	for down: bool in [true, false]:
		var event: InputEventMouseButton = InputEventMouseButton.new()
		event.window_id = root.get_window_id()
		event.position = point
		event.global_position = point
		event.button_index = button_index
		event.button_mask = MOUSE_BUTTON_MASK_LEFT if down and button_index == MOUSE_BUTTON_LEFT else 0
		event.pressed = down
		Input.parse_input_event(event)
		Input.flush_buffered_events()
		await process_frame
	await process_frame


func _run() -> void:
	root.size = Vector2i(1280, 800)
	root.gui_embed_subwindows = true
	for count: int in [1, 0]:
		_simulation = BgrSimulation.new()
		_check(bool(_simulation.configure(827364, 1200, 800, 0).get("ok", false)), "Configure retained native Statistics fixture")
		if count > 0:
			var draft: BgrGenomeDraft = _simulation.open_laboratory("empty")["draft"]
			_check(bool(draft.edit_rows("insert", 0).get("ok", false)), "Create the source laboratory gene fixture")
			_check(bool(_simulation.laboratory_set_property(draft, "symmetry", 1).get("ok", false)), "Set fixture symmetry through native laboratory validation")
			_check(bool(_simulation.commit_laboratory(draft).get("ok", false)), "Commit the fixture genotype")
			_check(bool(_simulation.paste_clipboard(600, 400).get("placement_succeeded", false)), "Place one fixture organism")
			_check(bool(_simulation.select_at(600, 400, false).get("selection_hit", false)), "Select the fixture parent")
			for feeding: int in range(32):
				_check(bool(_simulation.organism_action("feed").get("ok", false)), "Feed the native fixture without advancing ticks")
			_check(bool(_simulation.organism_action("reproduce").get("ok", false)), "Publish a real offspring record")
		var before_path: String = ProjectSettings.globalize_path("user://world-before-%d.bgw" % count)
		var packet: Dictionary = _simulation.statistics_open()
		_window = StatisticsWindow.new()
		_window.simulation = _simulation
		_window.initial_packet = packet
		_window.copied.connect(func(_result: Dictionary) -> void: _copies += 1)
		root.add_child(_window)
		await process_frame
		await process_frame
		_check(bool(_simulation.save_world_file(before_path).get("ok", false)), "Save baseline after native Statistics setup")
		var before_bytes: PackedByteArray = FileAccess.get_file_as_bytes(before_path)
		var previews: Array = _simulation.statistics_previews()["previews"]
		if count > 0:
			_check(bool(previews[0]["present"]), "Nonempty fixture provides an actual retained living champion")
		for index: int in range(6):
			var button: Button = _window.find_child("Record%dCopy" % index, true, false) as Button
			_check(button != null, "Record %d has an explicit primary-pointer Copy button" % index)
			if button == null:
				_finish()
				return
			_check(button.text == "Copy" and button.size.x >= 48 and button.size.y >= 48, "Record %d has a visible named touch target" % index)
			var present: bool = bool(previews[index]["present"])
			_check(button.disabled == (not present), "Record %d eligibility follows the retained native preview" % index)
			var expected: Dictionary = {}
			if present:
				_check(bool(_simulation.statistics_copy_record(index).get("ok", false)), "Native record %d supplies the expected genotype" % index)
				expected = _clipboard()
			var prior: int = _copies
			await _pointer(button)
			_check(_copies == prior + (1 if present else 0), "One primary-pointer action copies record %d once or respects absence" % index)
			if present:
				_check(_clipboard() == expected, "Record %d primary-pointer Copy preserves exact native genes and properties" % index)
				var preview: Control = _window.find_child("Record%dPreview" % index, true, false) as Control
				await _pointer(preview, MOUSE_BUTTON_RIGHT)
				var popup: PopupMenu = preview.get_node("PreviewCopyPopup") as PopupMenu
				_check(popup.visible and popup.get_item_text(0) == "Copy", "Record %d retains the right-click Copy menu" % index)
				popup.hide()
				preview.grab_focus()
				prior = _copies
				var key: InputEventKey = InputEventKey.new()
				key.keycode = KEY_ENTER
				key.pressed = true
				_window.push_input(key, true)
				_check(_copies == prior + 1 and _clipboard() == expected, "Record %d retains keyboard Copy and exact genotype" % index)
		var after_path: String = ProjectSettings.globalize_path("user://world-after-%d.bgw" % count)
		_check(bool(_simulation.save_world_file(after_path).get("ok", false)), "Save world after Copy actions")
		_check(FileAccess.get_file_as_bytes(after_path) == before_bytes, "Copy input preserves complete saved world bytes and RNG")
		_window.close_statistics()
		_window = null
		await process_frame
	_finish()


func _finish() -> void:
	if is_instance_valid(_window):
		_window.close_statistics()
		_window = null
	print("BGR_STATISTICS_TOUCH_COPY " + JSON.stringify({"result": "PASS" if _failures.is_empty() else "FAIL", "checks": _checks, "failures": _failures, "copies": _copies, "scope": "Real native Statistics and public Window pointer/key dispatch; no physical-touch claim"}))
	quit(0 if _failures.is_empty() else 1)
