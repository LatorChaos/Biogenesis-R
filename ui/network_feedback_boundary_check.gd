extends SceneTree

const Labels = preload("res://ui/network_labels.gd")
const NetworkWindow = preload("res://ui/network_window.gd")
const Main = preload("res://ui/main.gd")
const MainLabels = preload("res://ui/main_labels.gd")
const LOCALES: Array[String] = ["en", "es", "ca", "de", "pt", "ru"]
const NOTICE_CODES: Array[String] = ["listening", "connecting", "connected", "closed", "closing", "all_closed", "cancelled"]
const ERROR_CODES: Array[String] = ["invalid_local_port", "invalid_remote_port", "invalid_address", "invalid_identity", "already_closed", "world_required", "closing_previous", "duplicate", "capacity", "worker_capacity", "refused", "listener_failed", "transfer_failed", "incoming_failed", "preferences_failed", "cleanup_failed", "operation_failed"]
const CONNECTED: Dictionary = {
	"en": "Connected with [::1]:4321.",
	"es": "Conectado con [::1]:4321.",
	"ca": "Connectat amb [::1]:4321.",
	"de": "Mit [::1]:4321 verbunden.",
	"pt": "Ligado a [::1]:4321.",
	"ru": "Соединение с [::1]:4321 установлено."
}

class FeedbackWindow extends NetworkWindow:
	func _ready() -> void:
		_build()
		apply_snapshot(initial_snapshot)
		_load_config()
		apply_locale(language)
		select_mode(initial_mode)
		set_process(false)

class FeedbackMain extends Main:
	func _ready() -> void:
		_error_label = Label.new()
		add_child(_error_label)
		_network_status_label = Label.new()
		add_child(_network_status_label)
		_state = {"initialized": true}
		set_process(false)

	func _exit_tree() -> void:
		_network_error_packet.clear()

var _checks: int = 0
var _failures: Array[String] = []


func _initialize() -> void:
	_run.call_deferred()


func _check(condition: bool, description: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(description)


func _packet(code: String, arguments: Dictionary = {}, detail: String = "", channel: String = "notice") -> Dictionary:
	var packet: Dictionary = {
		"ok": channel != "error", "error": "", "notice": "",
		"error_feedback": {}, "notice_feedback": {}, "generation": 7,
		"settings": {"accept_connections": false, "local_port": 0, "max_connections": 1, "user_name": "Local"},
		"listening": false, "bound_port": 0, "job_limit": 4,
		"incoming_limit": 8, "connections": []
	}
	packet[channel] = "raw compatibility diagnostic"
	packet[channel + "_feedback"] = {"code": code, "arguments": arguments, "detail": detail}
	return packet


func _arguments(code: String) -> Dictionary:
	if code == "listening":
		return {"port": 65535, "limit": 8}
	if code in ["connecting", "connected", "closed"]:
		return {"address": "[::1]:4321"}
	if code == "refused":
		return {"reply": 16}
	return {}


func _run() -> void:
	for locale: String in LOCALES:
		for code: String in NOTICE_CODES + ERROR_CODES:
			var channel: String = "notice" if code in NOTICE_CODES else "error"
			var detail: String = "opaque diagnostic: Deadline\nport=4321" if code.ends_with("_failed") else ""
			var packet: Dictionary = _packet(code, _arguments(code), detail, channel)
			var text: String = Labels.feedback(packet, channel, locale)
			_check(not text.is_empty() and not text.contains("!"), locale + ": format " + code)
			_check(not text.contains("raw compatibility diagnostic"), locale + ": typed feedback replaces raw field " + code)
			if locale != "en":
				_check(text != Labels.feedback(packet, channel, "en"), locale + ": localized guidance " + code)
			if not detail.is_empty():
				_check(text.ends_with(detail), locale + ": preserve opaque diagnostic " + code)
			if code == "listening":
				_check(text.contains("65535") and text.contains("8"), locale + ": listener arguments")
			elif code == "refused":
				_check(text.contains("16"), locale + ": protocol reply argument")
		_check(Labels.feedback(_packet("connected", {"address": "[::1]:4321"}), "notice", locale) == CONNECTED[locale], locale + ": independent exact connected text")
		var empty: Dictionary = {"notice": "", "notice_feedback": {}}
		_check(Labels.feedback(empty, "notice", locale).is_empty(), locale + ": explicit empty feedback")
		var malformed: Array[Dictionary] = []
		var bad: Dictionary = _packet("connected", {"address": "[::1]:4321"})
		bad.erase("notice_feedback")
		malformed.append(bad)
		bad = _packet("connected", {"address": "[::1]:4321"})
		bad["notice_feedback"] = "wrong type"
		malformed.append(bad)
		malformed.append(_packet("unknown"))
		malformed.append(_packet("connected"))
		malformed.append(_packet("connected", {"address": 4321}))
		malformed.append(_packet("connected", {"address": ""}))
		malformed.append(_packet("connected", {"address": "x", "extra": 1}))
		malformed.append(_packet("listening", {"port": -1, "limit": 8}))
		malformed.append(_packet("listening", {"port": 65536, "limit": 8}))
		malformed.append(_packet("listening", {"port": 0.0, "limit": 8}))
		malformed.append(_packet("listening", {"port": 0, "limit": 0}))
		malformed.append(_packet("refused", {"reply": "16"}, "", "error"))
		malformed.append(_packet("refused", {"reply": 2147483648}, "", "error"))
		malformed.append(_packet("refused", {"reply": -2147483649}, "", "error"))
		malformed.append(_packet("connected", {"address": "x"}, "", "error"))
		malformed.append(_packet("operation_failed", {}, "", "notice"))
		malformed.append(_packet("connected", {"address": "x"}, "unexpected detail"))
		bad = _packet("connected", {"address": "x"})
		bad["notice_feedback"] = {}
		malformed.append(bad)
		bad = _packet("connected", {"address": "x"})
		bad["notice_feedback"]["arguments"] = []
		malformed.append(bad)
		bad = _packet("connected", {"address": "x"})
		bad["notice_feedback"]["code"] = 1
		malformed.append(bad)
		bad = _packet("connected", {"address": "x"})
		bad["notice_feedback"].erase("detail")
		malformed.append(bad)
		bad = _packet("connected", {"address": "x"})
		bad["notice_feedback"]["detail"] = 1
		malformed.append(bad)
		bad = _packet("connected", {"address": "x"})
		bad["notice_feedback"]["extra"] = 1
		malformed.append(bad)
		bad = _packet("connected", {"address": "x"})
		bad["notice"] = ""
		malformed.append(bad)
		for invalid: Dictionary in malformed:
			var channel: String = "notice" if bool(invalid["ok"]) else "error"
			var text: String = Labels.feedback(invalid, channel, locale)
			_check(text.begins_with(Labels.FEEDBACK[locale]["invalid_feedback"]), locale + ": reject malformed descriptor")
			var raw: String = invalid[channel]
			_check(raw.is_empty() or text.ends_with(raw), locale + ": retain malformed packet diagnostic")
	_check_windows_and_main()
	_check_capacity_labels()
	_check_error_payload_lifetime()
	_check_native_validation()
	print(JSON.stringify({"checks": _checks, "failures": _failures, "ok": _failures.is_empty()}))
	quit(0 if _failures.is_empty() else 1)


func _check_windows_and_main() -> void:
	var packet: Dictionary = _packet("connected", {"address": "[::1]:4321"})
	var first: FeedbackWindow = FeedbackWindow.new()
	var second: FeedbackWindow = FeedbackWindow.new()
	for window: FeedbackWindow in [first, second]:
		window.initial_snapshot = packet
		window.initial_mode = 1
		root.add_child(window)
	first._host.text = "first.invalid"
	second._host.text = "second.invalid"
	first._remote_port.text = "1234"
	second._remote_port.text = "5678"
	first._show_message("R_INVALID_PORT")
	var main: FeedbackMain = FeedbackMain.new()
	root.add_child(main)
	var failure: Dictionary = _packet("preferences_failed", {}, "opaque save diagnostic", "error")
	main._show_error(failure["error"], "", failure)
	main._network_packet = packet
	for locale: String in LOCALES:
		first.apply_locale(locale)
		second.apply_locale(locale)
		_check(first._feedback.text == Labels.message("R_INVALID_PORT", locale), locale + ": local validation wins")
		_check(second._feedback.text == CONNECTED[locale], locale + ": backend locale refresh without new packet")
		_check(first._host.text == "first.invalid" and second._host.text == "second.invalid", locale + ": independent edited hosts")
		_check(first._remote_port.text == "1234" and second._remote_port.text == "5678", locale + ": independent edited ports")
		main._main_language = locale
		main._refresh_feedback_locale()
		main._refresh_network_status()
		_check(main._error_label.text == MainLabels.message("R_ERROR", locale) % Labels.feedback(failure, "error", locale), locale + ": retained Main error locale")
		_check(main._network_status_label.text == CONNECTED[locale], locale + ": Main notice locale")
	main._show_error("unrelated error")
	_check(main._network_error_packet.is_empty(), "unrelated Main errors replace retained network feedback")
	first.free()
	second.free()
	main.free()


func _check_capacity_labels() -> void:
	var packet: Dictionary = _packet("connected", {"address": "[::1]:4321"})
	packet["notice"] = ""
	packet["notice_feedback"] = {}
	packet["settings"]["max_connections"] = 17
	for index: int in range(18):
		packet["connections"].append({"id": index + 1, "address": "127.0.0.1", "port": 30000 + index,
			"state": 51, "queued_incoming": 0, "incoming_candidate": false, "pending_outgoing": 0})
	var window: FeedbackWindow = FeedbackWindow.new()
	window.initial_snapshot = packet
	window.initial_mode = 1
	root.add_child(window)
	var main: FeedbackMain = FeedbackMain.new()
	root.add_child(main)
	main._network_packet = packet
	for locale: String in LOCALES:
		window.apply_locale(locale)
		main._main_language = locale
		main._refresh_network_status()
		var connections: String = Labels.message("R_CONNECTIONS", locale) % [18, 17]
		var limits: String = Labels.message("R_LIMITS", locale) % [4, 8]
		_check(window._status.text.ends_with(connections), locale + ": manager admission comes from configured maximum")
		_check(main._network_status_label.text == connections, locale + ": Main can show peers above incoming limit")
		_check(window._limits.text == limits and main._network_status_label.tooltip_text == limits, locale + ": outgoing request and arrival bounds")
		_check(connections.contains("18") and connections.contains("17") and not connections.contains("16"), locale + ": no fixed peer ceiling")
		if locale == "en":
			_check(connections == "Connections: 18; incoming connection limit: 17", "English incoming admission wording")
			_check(limits == "Limits: 4 outgoing network requests at once and 8 waiting arrivals per connection.", "English player resource wording")
		else:
			_check(connections != Labels.message("R_CONNECTIONS", "en") % [18, 17], locale + ": translated admission label")
			_check(limits != Labels.message("R_LIMITS", "en") % [4, 8], locale + ": translated resource label")
	window.free()
	main.free()


func _check_native_validation() -> void:
	var simulation: BgrSimulation = BgrSimulation.new()
	var initial: Dictionary = simulation.network_snapshot()
	_check(initial["job_limit"] == 4 and initial["incoming_limit"] == 8, "native resource bounds unchanged")
	_check(initial["settings"].has("max_connections") and not initial.has("runtime_peer_limit"), "native snapshot uses configured admission")
	_check(initial["error_feedback"].is_empty() and initial["notice_feedback"].is_empty(), "native empty descriptors")
	var cases: Array[Dictionary] = [
		{"packet": simulation.network_connect("127.0.0.1", "0"), "code": "invalid_remote_port", "raw": "Remote port must be a positive Java integer between 1 and 65535"},
		{"packet": simulation.network_connect("not-an-ip", "4321"), "code": "invalid_address", "raw": "A resolved IP address is required"},
		{"packet": simulation.network_connect("127.0.0.1", "4321"), "code": "world_required", "raw": "Initialize a world before connecting"},
		{"packet": simulation.network_disconnect(0), "code": "invalid_identity", "raw": "A valid connection identity is required"},
		{"packet": simulation.network_disconnect(1), "code": "already_closed", "raw": "This connection has already closed"}
	]
	for test: Dictionary in cases:
		var packet: Dictionary = test["packet"]
		_check(not bool(packet["ok"]) and packet["error"] == test["raw"], "native raw compatibility " + str(test["code"]))
		_check(packet["error_feedback"] == {"code": test["code"], "arguments": {}, "detail": ""}, "native exact descriptor " + str(test["code"]))
		for locale: String in LOCALES:
			_check(Labels.feedback(packet, "error", locale) == Labels.FEEDBACK[locale][test["code"]], locale + ": native validation formatting")


func _show_error_with_unrelated_payload(main: FeedbackMain) -> WeakRef:
	var payload: RefCounted = RefCounted.new()
	var reference: WeakRef = weakref(payload)
	var packet: Dictionary = _packet("preferences_failed", {}, "retained save diagnostic", "error")
	packet["world_snapshot"] = {"unrelated_payload": payload}
	packet["presentation"] = [payload]
	main._show_error(packet["error"], "", packet)
	_check(reference.get_ref() != null, "caller still owns unrelated packet payload")
	return reference


func _check_error_payload_lifetime() -> void:
	var main: FeedbackMain = FeedbackMain.new()
	root.add_child(main)
	var reference: WeakRef = _show_error_with_unrelated_payload(main)
	_check(reference.get_ref() == null, "retained network error releases unrelated packet payload after caller scope ends")
	main._main_language = "ru"
	main._refresh_feedback_locale()
	var expected: String = Labels.feedback(_packet("preferences_failed", {}, "retained save diagnostic", "error"), "error", "ru")
	_check(main._error_label.text == MainLabels.message("R_ERROR", "ru") % expected, "released payload does not prevent retained error locale refresh")
	main.free()
