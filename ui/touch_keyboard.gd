extends Node

const LETTER_ROWS: Array[String] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"]
const NUMBER_ROWS: Array[String] = ["789", "456", "123", "-0.+e"]
const SYMBOL_ROWS: Array[String] = ["!@#$%^&*()_+=", "[]{}<>:;?/,.", "\\|~`'\""]

var _enabled: bool = OS.get_name() == "Linux"
var _editor: WeakRef
var _layer: CanvasLayer
var _panel: PanelContainer
var _keys: VBoxContainer
var _mode: String = "letters"
var _shift: bool = false
var _key_height: float = 48.0
var _compact: bool = false


func _ready() -> void:
	get_tree().node_added.connect(_register)
	_register_tree(get_tree().root)


func set_enabled(value: bool) -> void:
	_enabled = value
	if not value:
		hide_keyboard()
	else:
		var editor: Control = _active_editor()
		if is_instance_valid(editor) and editor.has_focus():
			_focus_editor(editor)


func is_enabled() -> bool:
	return _enabled


func hide_keyboard() -> void:
	if is_instance_valid(_panel):
		_panel.hide()


func _register_tree(node: Node) -> void:
	_register(node)
	for child: Node in node.get_children():
		_register_tree(child)


func _register(node: Node) -> void:
	if node is Window:
		var window: Window = node as Window
		var restore: Callable = _restore_window.bind(window)
		if not window.focus_entered.is_connected(restore):
			window.focus_entered.connect(restore)
		var leave: Callable = _window_left.bind(window)
		if not window.focus_exited.is_connected(leave):
			window.focus_exited.connect(leave)
	if not (node is LineEdit or node is TextEdit):
		return
	var editor: Control = node as Control
	var enter: Callable = _focus_editor.bind(editor)
	if editor.focus_entered.is_connected(enter):
		return
	editor.focus_entered.connect(enter)
	editor.focus_exited.connect(_focus_left.bind(editor))
	editor.gui_input.connect(_editor_pointer.bind(editor))
	editor.tree_exiting.connect(_editor_removed.bind(editor))


func _active_editor() -> Control:
	return _editor.get_ref() as Control if _editor != null else null


func _eligible(editor: Control) -> bool:
	return _enabled and not DisplayServer.has_feature(DisplayServer.FEATURE_VIRTUAL_KEYBOARD) 		and is_instance_valid(editor) and editor.is_inside_tree() and editor.is_visible_in_tree() 		and (editor is LineEdit or editor is TextEdit) and bool(editor.get("editable")) 		and bool(editor.get("virtual_keyboard_enabled"))


func _restore_window(window: Window) -> void:
	var focused: Control = window.gui_get_focus_owner()
	if focused is LineEdit or focused is TextEdit:
		_focus_editor(focused)


func _window_left(window: Window) -> void:
	var editor: Control = _active_editor()
	if is_instance_valid(editor) and editor.get_window() == window:
		hide_keyboard()


func _focus_editor(editor: Control) -> void:
	if not _eligible(editor):
		hide_keyboard()
		return
	var window: Window = editor.get_window()
	if editor.get_viewport() != window:
		return
	var changed: bool = _active_editor() != editor
	_editor = weakref(editor)
	if not changed and is_instance_valid(_panel):
		_attach(window)
		_panel.show()
		_layout.call_deferred()
		return
	if editor is LineEdit and editor.virtual_keyboard_type in [DisplayServer.KEYBOARD_TYPE_NUMBER, DisplayServer.KEYBOARD_TYPE_NUMBER_DECIMAL]:
		_mode = "numbers"
	else:
		_mode = "letters"
	_shift = false
	_attach(window)
	_rebuild_keys()
	_panel.show()
	_layout.call_deferred()


func _focus_left(editor: Control) -> void:
	_check_focus.call_deferred(weakref(editor))


func _check_focus(reference: WeakRef) -> void:
	var previous: Control = reference.get_ref() as Control
	if _active_editor() == previous and (not is_instance_valid(previous) or not previous.has_focus()):
		hide_keyboard()


func _editor_removed(editor: Control) -> void:
	if _active_editor() == editor:
		hide_keyboard()
		_editor = null


func _editor_pointer(event: InputEvent, editor: Control) -> void:
	var pressed: bool = (event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT) 		or (event is InputEventScreenTouch and event.pressed and not event.canceled)
	if pressed and editor.has_focus() and (not is_instance_valid(_panel) or not _panel.visible):
		_show_focused.call_deferred(weakref(editor))


func _show_focused(reference: WeakRef) -> void:
	var editor: Control = reference.get_ref() as Control
	if is_instance_valid(editor) and editor.has_focus():
		_focus_editor(editor)


func _attach(window: Window) -> void:
	if not is_instance_valid(_layer):
		_layer = CanvasLayer.new()
		_layer.name = "TouchKeyboardLayer"
		_layer.layer = 100
		window.add_child(_layer)
		_panel = PanelContainer.new()
		_panel.name = "TouchKeyboard"
		_panel.mouse_filter = Control.MOUSE_FILTER_STOP
		_layer.add_child(_panel)
		var style: StyleBoxFlat = StyleBoxFlat.new()
		style.bg_color = Color(0.025, 0.045, 0.06, 0.98)
		style.border_color = Color(0.3, 0.5, 0.52, 0.8)
		style.set_border_width_all(1)
		style.set_corner_radius_all(10)
		style.content_margin_left = 10.0
		style.content_margin_right = 10.0
		style.content_margin_top = 8.0
		style.content_margin_bottom = 8.0
		_panel.add_theme_stylebox_override("panel", style)
		_keys = VBoxContainer.new()
		_keys.add_theme_constant_override("separation", 4)
		_panel.add_child(_keys)
	elif _layer.get_parent() != window:
		var previous: Window = _layer.get_parent() as Window
		if previous != null and previous.size_changed.is_connected(_layout):
			previous.size_changed.disconnect(_layout)
		_layer.reparent(window, false)
	if not window.size_changed.is_connected(_layout):
		window.size_changed.connect(_layout)


func _button(row: HBoxContainer, caption: String, node_name: String, action: Callable, width: float = 48.0) -> void:
	var button: Button = Button.new()
	button.name = node_name
	button.text = caption
	button.accessibility_name = caption
	button.custom_minimum_size = Vector2(width * (0.85 if _compact else 1.0), _key_height)
	if _compact:
		button.add_theme_font_size_override("font_size", 14)
	button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	button.focus_mode = Control.FOCUS_NONE
	button.pressed.connect(action)
	row.add_child(button)


func _row() -> HBoxContainer:
	var row: HBoxContainer = HBoxContainer.new()
	row.add_theme_constant_override("separation", 4)
	_keys.add_child(row)
	return row


func _active_rows() -> Array[String]:
	return LETTER_ROWS if _mode == "letters" else (NUMBER_ROWS if _mode == "numbers" else SYMBOL_ROWS)


func _update_key_metrics(row_count: int) -> bool:
	var extent: Vector2 = (_layer.get_parent() as Window).get_visible_rect().size
	var height: float = clampf(floorf((extent.y * 0.5 - 48.0 - 4.0 * row_count) / float(row_count + 1)), 32.0, 48.0)
	var compact: bool = extent.x < 720.0 or height < 42.0
	var changed: bool = height != _key_height or compact != _compact
	_key_height = height
	_compact = compact
	return changed


func _rebuild_keys() -> void:
	for child: Node in _keys.get_children():
		_keys.remove_child(child)
		child.queue_free()
	var rows: Array[String] = _active_rows()
	_update_key_metrics(rows.size())
	for letters: String in rows:
		var row: HBoxContainer = _row()
		for index: int in letters.length():
			var character: String = letters.substr(index, 1)
			if _shift:
				character = character.to_upper()
			_button(row, character, "TouchCharacter" + str(character.unicode_at(0)), _character.bind(character))
		if letters == rows[-1]:
			_button(row, "Space", "TouchSpace", _character.bind(" "), 76.0)
	var controls: HBoxContainer = _row()
	var next_mode: String = "numbers" if _mode == "letters" else ("symbols" if _mode == "numbers" else "letters")
	_button(controls, "123" if next_mode == "numbers" else ("#+=" if next_mode == "symbols" else "ABC"), "TouchMode", _set_mode.bind(next_mode), 54.0)
	_button(controls, "Shift", "TouchShift", _toggle_shift, 58.0)
	_button(controls, "All", "TouchSelectAll", _key.bind(KEY_A, 0, true), 48.0)
	_button(controls, "<", "TouchLeft", _key.bind(KEY_LEFT, 0, false))
	_button(controls, ">", "TouchRight", _key.bind(KEY_RIGHT, 0, false))
	_button(controls, "Backspace", "TouchBackspace", _key.bind(KEY_BACKSPACE, 0, false), 92.0)
	_button(controls, "Enter", "TouchEnter", _key.bind(KEY_ENTER, 0, false), 68.0)
	_button(controls, "Cancel", "TouchCancel", _cancel, 76.0)
	_button(controls, "Hide", "TouchHide", hide_keyboard, 64.0)


func _toggle_shift() -> void:
	_shift = not _shift
	_rebuild_keys()
	_layout.call_deferred()


func _set_mode(mode: String) -> void:
	_mode = mode
	_rebuild_keys()
	_layout.call_deferred()


func _character(value: String) -> void:
	_key(KEY_NONE, value.unicode_at(0), false)


func _key(code: Key, unicode: int, control: bool) -> void:
	_deliver_key.call_deferred(code, unicode, control)


func _deliver_key(code: Key, unicode: int, control: bool) -> void:
	var editor: Control = _active_editor()
	if not _eligible(editor) or not editor.has_focus():
		return
	var window: Window = editor.get_window()
	if window.gui_get_focus_owner() != editor:
		return
	for pressed: bool in [true, false]:
		if not is_instance_valid(window) or not window.is_inside_tree():
			break
		var event: InputEventKey = InputEventKey.new()
		event.window_id = window.get_window_id()
		event.keycode = code
		event.physical_keycode = code
		event.unicode = unicode if pressed else 0
		event.pressed = pressed
		event.ctrl_pressed = control
		event.shift_pressed = _shift and code in [KEY_LEFT, KEY_RIGHT]
		Input.parse_input_event(event)
	if code == KEY_ENTER and editor is LineEdit:
		hide_keyboard()


func _cancel() -> void:
	_key(KEY_ESCAPE, 0, false)
	hide_keyboard()


func _layout() -> void:
	var editor: Control = _active_editor()
	if not is_instance_valid(_panel) or not is_instance_valid(editor):
		return
	if _update_key_metrics(_active_rows().size()):
		_rebuild_keys()
		_layout.call_deferred()
		return
	var window: Window = editor.get_window()
	var extent: Vector2 = window.get_visible_rect().size
	_panel.size = Vector2(maxf(0.0, extent.x - 16.0), 0.0)
	var height: float = _panel.get_combined_minimum_size().y
	var top: bool = editor.get_global_rect().get_center().y > extent.y * 0.5
	_panel.position = Vector2(8.0, 8.0 if top else maxf(8.0, extent.y - height - 8.0))


func _exit_tree() -> void:
	if is_instance_valid(_layer):
		_layer.queue_free()
