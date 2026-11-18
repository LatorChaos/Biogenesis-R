extends Window

signal changed(result: Dictionary)

const Labels = preload("res://ui/network_labels.gd")
const LOOKUP_TIMEOUT_USEC: int = 10_000_000

var simulation: BgrSimulation
var language: String = "en"
var initial_mode: int = 0
var initial_snapshot: Dictionary = {}
var _tabs: TabContainer
var _accept: CheckBox
var _local_port: LineEdit
var _maximum: LineEdit
var _user_name: LineEdit
var _host: LineEdit
var _remote_port: LineEdit
var _connect: Button
var _cancel_lookup: Button
var _apply: Button
var _close: Button
var _rows: VBoxContainer
var _status: Label
var _limits: Label
var _feedback: Label
var _field_labels: Dictionary = {}
var _snapshot: Dictionary = {}
var _rows_cache: Array = []
var _peer_buttons: Dictionary = {}
var _resolver_id: int = IP.RESOLVER_INVALID_ID
var _resolve_deadline: int = 0
var _resolve_generation: int = -1
var _resolve_port: String = ""
var _feedback_key: String = ""
var _closing: bool = false
var _user_name_edited: bool = false


func _ready() -> void:
	transient = true
	exclusive = initial_mode == 0
	min_size = Vector2i(620, 440)
	size = Vector2i(720, 520)
	close_requested.connect(close_window)
	window_input.connect(_on_window_input)
	_build()
	apply_snapshot(initial_snapshot)
	initial_snapshot = {}
	_load_config()
	_user_name.text_changed.connect(_user_name_changed)
	apply_locale(language)
	select_mode(initial_mode)
	popup_centered_clamped(size)
	(_local_port if initial_mode == 0 else _host).grab_focus()


func _build() -> void:
	var margin: MarginContainer = MarginContainer.new()
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for side: String in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 12)
	add_child(margin)
	var column: VBoxContainer = VBoxContainer.new()
	column.add_theme_constant_override("separation", 10)
	margin.add_child(column)
	_status = Label.new()
	_status.name = "NetworkListenerStatus"
	_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	column.add_child(_status)
	_limits = Label.new()
	_limits.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	column.add_child(_limits)
	_tabs = TabContainer.new()
	_tabs.name = "NetworkTabs"
	_tabs.tabs_visible = false
	_tabs.size_flags_vertical = Control.SIZE_EXPAND_FILL
	column.add_child(_tabs)
	var config: VBoxContainer = VBoxContainer.new()
	config.name = "NetworkConfiguration"
	config.add_theme_constant_override("separation", 12)
	_tabs.add_child(config)
	_accept = CheckBox.new()
	_accept.name = "NetworkAcceptConnections"
	config.add_child(_accept)
	_local_port = _field(config, "T_LOCAL_PORT_TO_RECEIVE_CONNECTIONS", "NetworkLocalPort")
	_maximum = _field(config, "T_MAXIMUM_NUMBER_OF_ALLOWED_CONNECTIONS", "NetworkMaximumConnections")
	_user_name = _field(config, "T_USER_NAME", "NetworkUserName")
	var connections: VBoxContainer = VBoxContainer.new()
	connections.name = "NetworkConnections"
	connections.add_theme_constant_override("separation", 8)
	_tabs.add_child(connections)
	var form: HBoxContainer = HBoxContainer.new()
	connections.add_child(form)
	_host = _field(form, "T_IP", "NetworkHost")
	_host.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_remote_port = _field(form, "T_PORT", "NetworkRemotePort")
	_remote_port.text = "8888"
	_remote_port.custom_minimum_size.x = 90
	_host.text_submitted.connect(_connect_submitted)
	_remote_port.text_submitted.connect(_connect_submitted)
	var actions: HBoxContainer = HBoxContainer.new()
	connections.add_child(actions)
	_connect = Button.new()
	_connect.name = "NetworkConnect"
	_connect.pressed.connect(_begin_connect)
	actions.add_child(_connect)
	_cancel_lookup = Button.new()
	_cancel_lookup.name = "NetworkCancelLookup"
	_cancel_lookup.pressed.connect(_cancel_lookup_clicked)
	_cancel_lookup.hide()
	actions.add_child(_cancel_lookup)
	var scroll: ScrollContainer = ScrollContainer.new()
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	connections.add_child(scroll)
	_rows = VBoxContainer.new()
	_rows.name = "NetworkPeerRows"
	_rows.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(_rows)
	_feedback = Label.new()
	_feedback.name = "NetworkFeedback"
	_feedback.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_feedback.add_theme_color_override("font_color", Color("edbd91"))
	_feedback.hide()
	column.add_child(_feedback)
	var buttons: HBoxContainer = HBoxContainer.new()
	buttons.alignment = BoxContainer.ALIGNMENT_END
	column.add_child(buttons)
	_apply = Button.new()
	_apply.name = "NetworkApply"
	_apply.pressed.connect(_submit_config)
	buttons.add_child(_apply)
	_close = Button.new()
	_close.name = "NetworkCancel"
	_close.pressed.connect(close_window)
	buttons.add_child(_close)
	_tabs.tab_changed.connect(_mode_changed)


func _field(parent: Container, key: String, node_name: String) -> LineEdit:
	var row: VBoxContainer = VBoxContainer.new()
	row.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	parent.add_child(row)
	var label: Label = Label.new()
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	row.add_child(label)
	var editor: LineEdit = LineEdit.new()
	editor.name = node_name
	editor.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(editor)
	_field_labels[key] = [label, editor]
	return editor


func select_mode(mode: int) -> void:
	initial_mode = clampi(mode, 0, 1)
	if _tabs != null:
		_tabs.current_tab = initial_mode
		_mode_changed(initial_mode)
		if visible:
			(_local_port if initial_mode == 0 else _host).grab_focus()


func _mode_changed(mode: int) -> void:
	_apply.visible = mode == 0
	title = Labels.message("T_NETWORK_CONFIGURATION" if mode == 0 else "T_NETWORK_CONNECTIONS", language)


func apply_locale(next_language: String) -> void:
	language = next_language
	_accept.text = Labels.message("T_ALLOW_CONNECTIONS_FROM_OTHER_USERS", language)
	_accept.accessibility_name = _accept.text
	for key: String in _field_labels:
		var parts: Array = _field_labels[key]
		(parts[0] as Label).text = Labels.message(key, language)
		(parts[1] as LineEdit).accessibility_name = Labels.message(key, language)
	_tabs.set_tab_title(0, Labels.message("T_NETWORK_CONFIGURATION", language))
	_tabs.set_tab_title(1, Labels.message("T_NETWORK_CONNECTIONS", language))
	_connect.text = Labels.message("T_NEW_CONNECTION", language)
	_apply.text = Labels.message("T_OK", language)
	_close.text = Labels.message("T_CANCEL", language)
	_cancel_lookup.text = Labels.message("R_CANCEL_LOOKUP", language)
	_mode_changed(_tabs.current_tab)
	_refresh_status()
	_rebuild_rows(true)
	if not _feedback_key.is_empty():
		_show_message(_feedback_key)
	elif not _snapshot.is_empty():
		_refresh_feedback(_snapshot)


func _load_config() -> void:
	var settings: Dictionary = _snapshot["settings"]
	_accept.set_pressed_no_signal(bool(settings["accept_connections"]))
	_local_port.text = str(int(settings["local_port"]))
	_maximum.text = str(int(settings["max_connections"]))
	_user_name.text = str(settings["user_name"])
	_user_name_edited = false


func apply_snapshot(packet: Dictionary) -> void:
	if _closing:
		return
	var feedback_changed: bool = _snapshot.is_empty() or str(packet["error"]) != str(_snapshot["error"]) or str(packet["notice"]) != str(_snapshot["notice"]) or packet.get("error_feedback") != _snapshot.get("error_feedback") or packet.get("notice_feedback") != _snapshot.get("notice_feedback")
	if _resolver_id != IP.RESOLVER_INVALID_ID and int(packet["generation"]) != _resolve_generation:
		_cancel_lookup_clicked()
	_snapshot = packet
	_refresh_status()
	_rebuild_rows()
	if feedback_changed and _feedback_key.is_empty():
		_refresh_feedback(packet)


func _refresh_status() -> void:
	if _snapshot.is_empty():
		return
	var peers: Array = _snapshot["connections"]
	var listener: String = Labels.message("R_LISTENING", language) % int(_snapshot["bound_port"]) if bool(_snapshot["listening"]) else Labels.message("R_STOPPED", language)
	_status.text = listener + "\n" + Labels.message("R_CONNECTIONS", language) % [peers.size(), maxi(0, int(_snapshot["settings"]["max_connections"]))]
	_limits.text = Labels.message("R_LIMITS", language) % [int(_snapshot["job_limit"]), int(_snapshot["incoming_limit"])]


func _rebuild_rows(force: bool = false) -> void:
	if _snapshot.is_empty():
		return
	var peers: Array = _snapshot["connections"]
	if not force and peers == _rows_cache:
		return
	var focused: Control = get_viewport().gui_get_focus_owner()
	var focused_id: int = int(focused.get_meta("network_peer", -1)) if focused != null else -1
	for child: Node in _rows.get_children():
		_rows.remove_child(child)
		child.queue_free()
	_peer_buttons.clear()
	_rows_cache = peers.duplicate(true)
	if peers.is_empty():
		var empty: Label = Label.new()
		empty.text = Labels.message("R_EMPTY", language)
		_rows.add_child(empty)
	for peer: Dictionary in peers:
		var row: HBoxContainer = HBoxContainer.new()
		_rows.add_child(row)
		var details: VBoxContainer = VBoxContainer.new()
		details.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		row.add_child(details)
		var address: Label = Label.new()
		var state: int = int(peer["state"])
		var state_text: String = Labels.message("R_CONNECTED" if state == 51 else ("R_DISCONNECTED" if state == 50 else "R_CONNECTING"), language)
		if state not in [50, 51, 52]:
			state_text = Labels.message("R_STATE", language) % state
		var host: String = str(peer["address"])
		address.text = "%s:%d - %s" % ["[" + host + "]" if ":" in host else host, int(peer["port"]), state_text]
		address.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		details.add_child(address)
		var queues: Label = Label.new()
		queues.text = Labels.message("R_QUEUES", language) % [int(peer["queued_incoming"]), Labels.message("T_YES" if bool(peer["incoming_candidate"]) else "T_NO", language), int(peer["pending_outgoing"])]
		queues.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		details.add_child(queues)
		var disconnect: Button = Button.new()
		var peer_id: int = int(peer["id"])
		disconnect.name = "NetworkDisconnect" + str(peer_id)
		disconnect.text = Labels.message("T_DISCONNECT", language)
		disconnect.accessibility_name = disconnect.text + " " + address.text
		disconnect.set_meta("network_peer", peer_id)
		disconnect.pressed.connect(_disconnect.bind(peer_id))
		row.add_child(disconnect)
		_peer_buttons[peer_id] = disconnect
	if _peer_buttons.has(focused_id):
		(_peer_buttons[focused_id] as Button).grab_focus()


func _submit_config() -> void:
	if _closing:
		return
	_cancel_resolution()
	var packet: Dictionary = simulation.network_apply(_accept.button_pressed, _local_port.text, _maximum.text, _user_name.text, _user_name_edited)
	_handle_result(packet)
	if bool(packet["ok"]):
		close_window()


func _user_name_changed(_text: String) -> void:
	_user_name_edited = true


func _disconnect(peer_id: int) -> void:
	_handle_result(simulation.network_disconnect(peer_id))


func _handle_result(packet: Dictionary) -> void:
	_refresh_feedback(packet)
	changed.emit(packet)


func _refresh_feedback(packet: Dictionary) -> void:
	_feedback_key = ""
	_feedback.text = Labels.feedback(packet, "error" if not bool(packet["ok"]) else "notice", language)
	_feedback.visible = not _feedback.text.is_empty()


func _connect_submitted(_text: String) -> void:
	_begin_connect()


func _begin_connect() -> void:
	if _closing or _resolver_id != IP.RESOLVER_INVALID_ID:
		return
	if not BgrSimulation.network_valid_remote_port(_remote_port.text):
		_show_message("R_INVALID_PORT")
		_remote_port.grab_focus()
		return
	if _host.text.is_empty():
		_show_message("R_HOST_REQUIRED")
		_host.grab_focus()
		return
	_resolver_id = IP.resolve_hostname_queue_item(_host.text, IP.TYPE_ANY)
	if _resolver_id == IP.RESOLVER_INVALID_ID:
		_show_message("R_DNS_UNAVAILABLE")
		return
	_resolve_port = _remote_port.text
	_resolve_generation = int(_snapshot["generation"])
	_resolve_deadline = Time.get_ticks_usec() + LOOKUP_TIMEOUT_USEC
	_connect.disabled = true
	_host.editable = false
	_remote_port.editable = false
	_cancel_lookup.show()
	_show_message("R_RESOLVING")


func _process(_delta: float) -> void:
	if _resolver_id == IP.RESOLVER_INVALID_ID or _closing:
		return
	if Time.get_ticks_usec() >= _resolve_deadline:
		_cancel_resolution()
		_show_message("R_DNS_TIMEOUT")
		return
	var status: int = IP.get_resolve_item_status(_resolver_id)
	if status == IP.RESOLVER_STATUS_WAITING:
		return
	var addresses: Array = IP.get_resolve_item_addresses(_resolver_id) if status == IP.RESOLVER_STATUS_DONE else []
	var port: String = _resolve_port
	_cancel_resolution()
	if status != IP.RESOLVER_STATUS_DONE or addresses.is_empty():
		_show_message("R_DNS_ERROR")
		return
	_handle_result(simulation.network_connect(str(addresses[0]), port))


func _show_message(key: String) -> void:
	_feedback_key = key
	_feedback.text = Labels.message(key, language)
	_feedback.show()


func _cancel_lookup_clicked() -> void:
	_cancel_resolution()
	_feedback_key = ""
	_feedback.hide()


func _cancel_resolution() -> void:
	if _resolver_id != IP.RESOLVER_INVALID_ID:
		IP.erase_resolve_item(_resolver_id)
	_resolver_id = IP.RESOLVER_INVALID_ID
	_resolve_generation = -1
	_resolve_port = ""
	if not _closing and is_instance_valid(_connect):
		_connect.disabled = false
		_host.editable = true
		_remote_port.editable = true
		_cancel_lookup.hide()


func close_window() -> void:
	if _closing:
		return
	_closing = true
	_cancel_resolution()
	hide()
	queue_free()


func _on_window_input(event: InputEvent) -> void:
	if not event is InputEventKey:
		return
	var key: InputEventKey = event as InputEventKey
	if not key.pressed or key.echo:
		return
	if key.keycode == KEY_ESCAPE:
		close_window()
		get_viewport().set_input_as_handled()
	elif key.alt_pressed and not key.ctrl_pressed and not key.meta_pressed:
		if key.keycode == Labels.mnemonic("T_CANCEL", language):
			close_window()
		elif _tabs.current_tab == 0 and key.keycode == Labels.mnemonic("T_OK", language):
			_submit_config()
		elif _tabs.current_tab == 1 and key.keycode == Labels.mnemonic("T_NEW_CONNECTION", language):
			_begin_connect()
		else:
			return
		get_viewport().set_input_as_handled()
	elif key.keycode in [KEY_ENTER, KEY_KP_ENTER] and _tabs.current_tab == 0:
		if get_viewport().gui_get_focus_owner() is LineEdit:
			_submit_config()
			get_viewport().set_input_as_handled()


func _exit_tree() -> void:
	_closing = true
	_cancel_resolution()
