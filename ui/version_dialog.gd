# Biogenesis (C) 2006-2010 Joan Queralt Molina
# SPDX-License-Identifier: GPL-2.0-or-later

extends AcceptDialog

signal closed

const HelpAbout = preload("res://ui/help_about.gd")
const SettingsFields = preload("res://ui/settings_fields.gd")
const StartupArguments = preload("res://ui/startup_arguments.gd")
const SOURCE_URL: String = "http://biogenesis.sourceforge.net/lastversion.txt"
const LOCAL_VERSION: String = "900"
const TIMEOUT_SECONDS: float = 10.0
const BODY_LIMIT_BYTES: int = 4096
const REDIRECT_LIMIT: int = 3

var language: String = "en"
var language_provider: Callable
var _request: HTTPRequest
var _text: Label
var _closing: bool = false
var _finished: bool = false


static func translate_version(text: String) -> String:
	if text.length() > 6:
		return text
	for index: int in range(text.length()):
		if text.unicode_at(index) > 0xffff:
			return text
	var padded: String = "000000".substr(0, 6 - text.length()) + text
	var parts: Array[int] = []
	for index: int in [0, 2, 4]:
		var parsed: Dictionary = StartupArguments.signed_long(padded.substr(index, 2))
		if not bool(parsed["ok"]):
			return text
		parts.append(int(parsed["value"]))
	var translated: String = str(parts[0]) + "." + str(parts[1])
	if parts[2] != 0:
		translated += "." + str(parts[2])
	return translated


func _ready() -> void:
	force_native = true
	transient = true
	exclusive = true
	get_label().hide()
	confirmed.connect(close_check)
	canceled.connect(close_check)
	var scroll: ScrollContainer = ScrollContainer.new()
	scroll.name = "VersionCheckScroll"
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	scroll.custom_minimum_size = Vector2(500, 120)
	scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_text = Label.new()
	_text.name = "VersionCheckText"
	_text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_text.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(_text)
	add_child(scroll)
	var locale: String = _current_language()
	title = HelpAbout.message("T_VERSION_TITLE", locale)
	_text.text = HelpAbout.message("T_CHECK_LAST_VERSION", locale)
	get_ok_button().name = "VersionCheckClose"
	get_ok_button().text = _button_text("T_CANCEL", locale)
	_request = HTTPRequest.new()
	_request.name = "VersionHTTPRequest"
	_request.timeout = TIMEOUT_SECONDS
	_request.body_size_limit = BODY_LIMIT_BYTES
	_request.max_redirects = REDIRECT_LIMIT
	_request.accept_gzip = false
	_request.request_completed.connect(_request_completed)
	add_child(_request)
	popup_centered_clamped(Vector2i(620, 260))
	get_ok_button().grab_focus()
	var error: Error = _request.request(_request_url())
	if error != OK:
		_show_failure("R_VERSION_START_ERROR", [int(error), error_string(error)])


func _request_url() -> String:
	return SOURCE_URL


func _current_language() -> String:
	return str(language_provider.call()) if language_provider.is_valid() else language


func _button_text(key: String, locale: String) -> String:
	var messages: Dictionary = SettingsFields.DATA["messages"]
	var current: Dictionary = messages.get(locale, messages["en"])
	return str(current[key])


func _request_completed(result: int, response_code: int, _headers: PackedStringArray, body: PackedByteArray) -> void:
	if _closing or _finished or not is_inside_tree():
		return
	if result != HTTPRequest.RESULT_SUCCESS:
		var key: String = "R_VERSION_TRANSPORT_ERROR"
		var arguments: Array = [result]
		if result == HTTPRequest.RESULT_TIMEOUT:
			key = "R_VERSION_TIMEOUT"
			arguments = [int(TIMEOUT_SECONDS)]
		elif result == HTTPRequest.RESULT_BODY_SIZE_LIMIT_EXCEEDED:
			key = "R_VERSION_BODY_LIMIT"
			arguments = [BODY_LIMIT_BYTES]
		elif result == HTTPRequest.RESULT_REDIRECT_LIMIT_REACHED:
			key = "R_VERSION_REDIRECT_LIMIT"
			arguments = [REDIRECT_LIMIT]
		_show_failure(key, arguments)
		return
	if response_code < 200 or response_code >= 400:
		_show_failure("R_VERSION_HTTP_ERROR", [response_code])
		return
	if body.is_empty():
		_show_failure("R_VERSION_EMPTY")
		return
	var end: int = body.size()
	for index: int in range(body.size()):
		if body[index] == 10 or body[index] == 13:
			end = index
			break
	var line: PackedByteArray = body.slice(0, end)
	if not _valid_utf8_line(line):
		_show_failure("R_VERSION_UTF8")
		return
	var text: String = line.get_string_from_utf8()
	if line.size() >= 3 and line[0] == 0xef and line[1] == 0xbb and line[2] == 0xbf:
		# Godot removes the UTF-8 BOM; the Source reader preserves it
		text = String.chr(0xfeff) + text
	var locale: String = _current_language()
	_finish(HelpAbout.message("T_YOUR_VERSION", locale) + translate_version(LOCAL_VERSION) + "\n" + HelpAbout.message("T_LAST_VERSION", locale) + translate_version(text), locale)


func _valid_utf8_line(bytes: PackedByteArray) -> bool:
	var index: int = 0
	while index < bytes.size():
		var first: int = bytes[index]
		if first == 0:
			return false
		if first < 0x80:
			index += 1
			continue
		var count: int = 0
		if first >= 0xc2 and first <= 0xdf:
			count = 1
		elif first >= 0xe0 and first <= 0xef:
			count = 2
		elif first >= 0xf0 and first <= 0xf4:
			count = 3
		if count == 0 or index + count >= bytes.size():
			return false
		for following: int in range(1, count + 1):
			if bytes[index + following] < 0x80 or bytes[index + following] > 0xbf:
				return false
		var second: int = bytes[index + 1]
		if (first == 0xe0 and second < 0xa0) or (first == 0xed and second >= 0xa0):
			return false
		if (first == 0xf0 and second < 0x90) or (first == 0xf4 and second >= 0x90):
			return false
		index += count + 1
	return true


func _show_failure(key: String, arguments: Array = []) -> void:
	var locale: String = _current_language()
	var detail: String = HelpAbout.message(key, locale)
	if not arguments.is_empty():
		detail = detail % arguments
	var prefix: String = HelpAbout.message("T_CANT_CHECK_CURRENT_VERSION", locale)
	_finish(prefix + ("" if prefix.ends_with(" ") else " ") + detail, locale)


func _finish(text: String, locale: String) -> void:
	if _closing or _finished:
		return
	_finished = true
	title = HelpAbout.message("T_VERSION_TITLE", locale)
	_text.text = text
	get_ok_button().text = _button_text("T_OK", locale)
	get_ok_button().grab_focus()


func close_check() -> void:
	if _closing:
		return
	_closing = true
	if is_instance_valid(_request):
		_request.cancel_request()
	hide()
	closed.emit()
	queue_free()


func _exit_tree() -> void:
	_closing = true
	if is_instance_valid(_request):
		_request.cancel_request()
