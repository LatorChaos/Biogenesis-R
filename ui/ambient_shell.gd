extends Control

signal pause_requested
signal cinema_toggled(enabled: bool)
signal fit_requested

const PAUSE_ICON: Texture2D = preload("res://ui/assets/pause.svg")
const RESUME_ICON: Texture2D = preload("res://ui/assets/resume.svg")
const LABELS: Dictionary = {
	"en": ["Tools", "Pause", "Resume", "Fit", "Cinema", "Close", "Selected organism"],
	"es": ["Herramientas", "Pausa", "Continuar", "Ajustar", "Cine", "Cerrar", "Organismo seleccionado"],
	"de": ["Werkzeuge", "Pause", "Fortsetzen", "Einpassen", "Kino", "Schließen", "Ausgewählter Organismus"],
	"pt": ["Ferramentas", "Pausar", "Retomar", "Ajustar", "Cinema", "Fechar", "Organismo selecionado"],
	"ru": ["Инструменты", "Пауза", "Продолжить", "Вписать", "Кино", "Закрыть", "Выбранный организм"],
	"ca": ["Eines", "Pausa", "Continua", "Ajusta", "Cinema", "Tanca", "Organisme seleccionat"]
}

var hold_open: bool = false
var idle_seconds: float = 5.0
var _canvas: Control
var _inspector: Control
var _hud: Control
var _dock: PanelContainer
var _drawer: PanelContainer
var _tools_scroll: ScrollContainer
var _inspector_panel: PanelContainer
var _inspector_scroll: ScrollContainer
var _pause: Button
var _tools_button: Button
var _fit: Button
var _cinema: Button
var _drawer_title: Label
var _inspector_title: Label
var _drawer_close: Button
var _language: String = "en"
var _language_applied: bool = false
var _paused: bool = true
var _idle: float = 0.0
var _idle_clock_usec: int = -1
var _opacity: float = 1.0
var _wake_pointer: bool = false
var _wake_on_next_press: bool = false
var _wake_after_pointer: bool = false
var _wake_touch_indices: Dictionary[int, bool] = {}
var _wake_mouse_pointer: bool = false
var _cursor_hidden_by_shell: bool = false


func _ready() -> void:
	_build()
	_idle_clock_usec = _clock_usec()
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	resized.connect(_layout)
	_layout()


## Reparent existing controls without changing their actions or simulation ownership
func configure(canvas: Control, tools: Control, inspector: Control) -> void:
	assert(_canvas == null and canvas != null and tools != null and inspector != null)
	assert(canvas != tools and tools != inspector and canvas != inspector)
	assert(not canvas.is_ancestor_of(self) and not tools.is_ancestor_of(self) and not inspector.is_ancestor_of(self))
	_build()
	_canvas = canvas
	_inspector = inspector
	_move(canvas, self)
	move_child(canvas, 0)
	canvas.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_move(inspector, _inspector_scroll)
	_move(tools, _tools_scroll)
	tools.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	inspector.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	inspector.visibility_changed.connect(_update_inspector)
	inspector.minimum_size_changed.connect(_layout)
	_update_inspector()
	_layout()


func set_paused(paused: bool) -> void:
	if _pause != null and _paused == paused and _pause.icon == (RESUME_ICON if paused else PAUSE_ICON) and _pause.text == str(LABELS[_language][2 if paused else 1]):
		return
	_paused = paused
	if _pause != null:
		_refresh_pause_caption()
		_layout()


func _refresh_pause_caption() -> void:
	_pause.icon = RESUME_ICON if _paused else PAUSE_ICON
	_pause.text = str(LABELS[_language][2 if _paused else 1])
	_pause.accessibility_name = _pause.text
	_pause.tooltip_text = _pause.text


func set_cinema_enabled(enabled: bool) -> void:
	_build()
	_cinema.set_pressed_no_signal(enabled)


func set_language(language: String) -> void:
	var selected: String = language if LABELS.has(language) else "en"
	if _language_applied and _language == selected:
		return
	_language = selected
	if _hud == null:
		return
	var words: Array = LABELS[_language]
	_tools_button.text = str(words[0])
	_tools_button.tooltip_text = str(words[0]) + " [F10]"
	_fit.text = str(words[3])
	_cinema.text = str(words[4])
	_drawer_title.text = str(words[0])
	_inspector_title.text = str(words[6])
	_drawer_close.text = str(words[5])
	for button: Button in [_tools_button, _fit, _cinema, _drawer_close]:
		button.accessibility_name = button.text
	_refresh_pause_caption()
	_language_applied = true
	_layout()


func reveal() -> void:
	if _hud == null:
		return
	_wake_on_next_press = false
	_show_hud()
	_update_inspector()


func is_idle() -> bool:
	return _hud != null and not _hud.visible and not hold_open


func get_inspector_rect() -> Rect2:
	if _inspector_panel != null and _inspector_panel.is_visible_in_tree():
		return _inspector_panel.get_global_rect()
	return Rect2()


func show_tools() -> void:
	reveal()
	_drawer.show()
	_update_inspector()
	_drawer_close.grab_focus()


func toggle_tools() -> void:
	if _drawer.visible:
		_drawer.hide()
		_update_inspector()
		_tools_button.grab_focus()
		_show_hud()
	else:
		show_tools()


func _move(control: Control, parent: Node) -> void:
	if control.get_parent() == null:
		parent.add_child(control)
	else:
		control.reparent(parent, false)


func _panel(node_name: String) -> PanelContainer:
	var panel: PanelContainer = PanelContainer.new()
	panel.name = node_name
	panel.mouse_filter = Control.MOUSE_FILTER_STOP
	var style: StyleBoxFlat = StyleBoxFlat.new()
	style.bg_color = Color(0.025, 0.045, 0.06, 0.9)
	style.border_color = Color(0.3, 0.5, 0.52, 0.7)
	style.set_border_width_all(1)
	style.set_corner_radius_all(14)
	style.content_margin_left = 12.0
	style.content_margin_right = 12.0
	style.content_margin_top = 10.0
	style.content_margin_bottom = 10.0
	panel.add_theme_stylebox_override("panel", style)
	_hud.add_child(panel)
	return panel


func _button(parent: Node, node_name: String, action: Callable) -> Button:
	var button: Button = Button.new()
	button.name = node_name
	button.custom_minimum_size = Vector2(48.0, 48.0)
	button.focus_mode = Control.FOCUS_ALL
	button.pressed.connect(action)
	parent.add_child(button)
	return button


func _content_panel(panel: PanelContainer, title_name: String) -> VBoxContainer:
	var column: VBoxContainer = VBoxContainer.new()
	column.add_theme_constant_override("separation", 8)
	panel.add_child(column)
	var header: HBoxContainer = HBoxContainer.new()
	column.add_child(header)
	var title: Label = Label.new()
	title.name = title_name
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title)
	return column


func _build() -> void:
	if _hud != null:
		return
	name = "AmbientShell"
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_hud = Control.new()
	_hud.name = "AmbientHud"
	_hud.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_hud)
	_hud.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_drawer = _panel("AmbientToolsDrawer")
	var column: VBoxContainer = _content_panel(_drawer, "AmbientToolsTitle")
	var header: HBoxContainer = column.get_child(0) as HBoxContainer
	_drawer_title = header.get_child(0) as Label
	_drawer_close = _button(header, "AmbientCloseTools", toggle_tools)
	_tools_scroll = ScrollContainer.new()
	_tools_scroll.name = "AmbientToolsScroll"
	_tools_scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	column.add_child(_tools_scroll)
	_drawer.hide()
	_inspector_panel = _panel("AmbientInspector")
	_inspector_panel.reparent(self, false)
	column = _content_panel(_inspector_panel, "AmbientInspectorTitle")
	header = column.get_child(0) as HBoxContainer
	_inspector_title = header.get_child(0) as Label
	_inspector_scroll = ScrollContainer.new()
	_inspector_scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	column.add_child(_inspector_scroll)
	_inspector_panel.hide()
	_dock = _panel("AmbientDock")
	var row: HBoxContainer = HBoxContainer.new()
	row.add_theme_constant_override("separation", 8)
	_dock.add_child(row)
	_pause = _button(row, "AmbientPause", func() -> void:
		_show_hud()
		pause_requested.emit())
	_tools_button = _button(row, "AmbientTools", toggle_tools)
	_fit = _button(row, "AmbientFit", func() -> void:
		_show_hud()
		fit_requested.emit())
	_cinema = _button(row, "AmbientCinema", func() -> void:
		_show_hud())
	_cinema.toggle_mode = true
	_cinema.toggled.connect(func(enabled: bool) -> void: cinema_toggled.emit(enabled))
	set_language(_language)


func _layout() -> void:
	if _dock == null:
		return
	_dock.size = _dock.get_combined_minimum_size()
	_dock.position = Vector2(maxf(8.0, (size.x - _dock.size.x) * 0.5), maxf(8.0, size.y - _dock.size.y - 16.0))
	_drawer.position = Vector2(16.0, 16.0)
	var shared_width: float = (size.x - 48.0) * 0.5 if _drawer.visible and _inspector_panel.visible else size.x - 32.0
	_drawer.size = Vector2(maxf(48.0, minf(760.0, shared_width)), maxf(120.0, size.y - _dock.size.y - 48.0))
	var inspector_height: float = 280.0
	if _inspector != null:
		inspector_height = _inspector_panel.get_combined_minimum_size().y - _inspector_scroll.get_combined_minimum_size().y + _inspector.get_combined_minimum_size().y
	_inspector_panel.size = Vector2(maxf(48.0, minf(600.0, shared_width)), maxf(120.0, minf(inspector_height, minf(280.0, size.y - _dock.size.y - 48.0))))
	_inspector_panel.position = Vector2(maxf(16.0, size.x - _inspector_panel.size.x - 16.0), 16.0)


func _update_inspector() -> void:
	if _inspector != null:
		_inspector_panel.visible = _inspector.visible
		_layout()


func _show_hud() -> void:
	_restore_cursor()
	_idle_clock_usec = _clock_usec()
	_idle = 0.0
	_opacity = 1.0
	_hud.modulate.a = 1.0
	_hud.mouse_behavior_recursive = Control.MOUSE_BEHAVIOR_INHERITED
	_hud.show()


func _clock_usec() -> int:
	return Time.get_ticks_usec()


func _process(_delta: float) -> void:
	if _hud == null:
		return
	var now: int = _clock_usec()
	var elapsed: float = float(maxi(0, now - _idle_clock_usec)) / 1000000.0 if _idle_clock_usec >= 0 else 0.0
	_idle_clock_usec = now
	_advance_idle(elapsed)


func _advance_idle(delta: float) -> void:
	if _hud == null or not is_finite(delta) or delta <= 0.0:
		return
	var focused: Control = get_viewport().gui_get_focus_owner()
	var editing: bool = focused is LineEdit or focused is TextEdit
	if hold_open or _drawer.visible or _wake_pointer or (editing and (_hud.is_ancestor_of(focused) or _inspector_panel.is_ancestor_of(focused))):
		_show_hud()
		return
	var elapsed: float = clampf(delta, 0.0, 0.25)
	_idle = minf(idle_seconds, _idle + delta)
	if _idle < idle_seconds:
		return
	_opacity = maxf(0.0, _opacity - elapsed * 4.0)
	_hud.modulate.a = _opacity
	if _opacity == 0.0:
		_wake_on_next_press = false
		_hud.mouse_behavior_recursive = Control.MOUSE_BEHAVIOR_DISABLED
		_hud.hide()
		if not _cursor_hidden_by_shell and DisplayServer.has_feature(DisplayServer.FEATURE_MOUSE) and get_window().has_focus() and Input.mouse_mode == Input.MOUSE_MODE_VISIBLE:
			Input.mouse_mode = Input.MOUSE_MODE_HIDDEN
			_cursor_hidden_by_shell = true


func _restore_cursor() -> void:
	if _cursor_hidden_by_shell:
		_cursor_hidden_by_shell = false
		if Input.mouse_mode == Input.MOUSE_MODE_HIDDEN:
			Input.mouse_mode = Input.MOUSE_MODE_VISIBLE


func _input(event: InputEvent) -> void:
	if _hud == null or not is_visible_in_tree():
		return
	if event is InputEventMouseButton or event is InputEventScreenTouch:
		var pressed: bool = (event as InputEventMouseButton).pressed if event is InputEventMouseButton else (event as InputEventScreenTouch).pressed
		if _wake_pointer:
			_wake_pointer = pressed
			get_viewport().set_input_as_handled()
			return
		if _wake_after_pointer:
			if event is InputEventScreenTouch and not _wake_mouse_pointer:
				var index: int = (event as InputEventScreenTouch).index
				if pressed:
					_wake_touch_indices[index] = true
				else:
					_wake_touch_indices.erase(index)
			elif event is InputEventMouseButton and _wake_mouse_pointer and not pressed:
				_wake_mouse_pointer = false
			if not _wake_mouse_pointer and _wake_touch_indices.is_empty():
				_wake_after_pointer = false
				reveal.call_deferred()
			return
		if pressed:
			var was_hidden: bool = not _hud.visible or _wake_on_next_press
			if was_hidden and not hold_open and _inspector_panel.is_visible_in_tree():
				_wake_after_pointer = true
				if event is InputEventScreenTouch:
					_wake_touch_indices[(event as InputEventScreenTouch).index] = true
				else:
					_wake_mouse_pointer = true
				return
			reveal()
			if was_hidden and not hold_open:
				_wake_pointer = true
				get_viewport().set_input_as_handled()
	elif event is InputEventMouseMotion or event is InputEventScreenDrag:
		if _wake_after_pointer:
			return
		if event is InputEventMouseMotion and not _hud.visible:
			_wake_on_next_press = true
		_show_hud()
	elif event is InputEventKey and event.pressed and not event.echo:
		var was_hidden: bool = not _hud.visible
		reveal()
		if hold_open:
			return
		if event.keycode == KEY_F10:
			toggle_tools()
			get_viewport().set_input_as_handled()
		elif event.keycode == KEY_ESCAPE and _drawer.visible:
			toggle_tools()
			get_viewport().set_input_as_handled()
		elif event.keycode == KEY_TAB and was_hidden:
			_tools_button.grab_focus()
			get_viewport().set_input_as_handled()


func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_WINDOW_FOCUS_OUT:
		_restore_cursor()
		_wake_pointer = false
		_wake_on_next_press = false
		_wake_after_pointer = false
		_wake_touch_indices.clear()
		_wake_mouse_pointer = false
	elif what == NOTIFICATION_WM_WINDOW_FOCUS_IN and _hud != null:
		_show_hud()


func _exit_tree() -> void:
	_restore_cursor()
