extends SceneTree

const VersionDialog = preload("res://ui/version_dialog.gd")
const HelpAbout = preload("res://ui/help_about.gd")
const LOCALES: Array[String] = ["en", "es", "ca", "de", "pt", "ru"]

class CallbackDialog extends VersionDialog:
	func _ready() -> void:
		_text = Label.new()
		add_child(_text)

var _language: String = "en"
var _checks: int = 0
var _closed: int = 0
var _failures: Array[String] = []
var _observations: Array[Dictionary] = []


func _initialize() -> void:
	_run.call_deferred()


func _current_language() -> String:
	return _language


func _check(condition: bool, description: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(description)


func _run() -> void:
	var expected: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://ui/version_feedback_expected.json"))
	var source: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://ui/version_check_expected.json"))["locales"]
	var cases: Array[Dictionary] = [
		{"key": "R_VERSION_TRANSPORT_ERROR", "result": HTTPRequest.RESULT_CANT_CONNECT, "http": 0, "body": PackedByteArray(), "arguments": [HTTPRequest.RESULT_CANT_CONNECT]},
		{"key": "R_VERSION_TIMEOUT", "result": HTTPRequest.RESULT_TIMEOUT, "http": 0, "body": PackedByteArray(), "arguments": [10]},
		{"key": "R_VERSION_BODY_LIMIT", "result": HTTPRequest.RESULT_BODY_SIZE_LIMIT_EXCEEDED, "http": 200, "body": PackedByteArray(), "arguments": [4096]},
		{"key": "R_VERSION_REDIRECT_LIMIT", "result": HTTPRequest.RESULT_REDIRECT_LIMIT_REACHED, "http": 302, "body": PackedByteArray(), "arguments": [3]},
		{"key": "R_VERSION_HTTP_ERROR", "result": HTTPRequest.RESULT_SUCCESS, "http": 503, "body": "900".to_utf8_buffer(), "arguments": [503]},
		{"key": "R_VERSION_EMPTY", "result": HTTPRequest.RESULT_SUCCESS, "http": 200, "body": PackedByteArray(), "arguments": []},
		{"key": "R_VERSION_UTF8", "result": HTTPRequest.RESULT_SUCCESS, "http": 200, "body": PackedByteArray([255, 10]), "arguments": []},
		{"key": "R_VERSION_START_ERROR", "arguments": [int(ERR_INVALID_PARAMETER), error_string(ERR_INVALID_PARAMETER)]}
	]
	for locale: String in LOCALES:
		for test: Dictionary in cases:
			var dialog: CallbackDialog = CallbackDialog.new()
			dialog.language = "en"
			dialog.language_provider = _current_language
			_language = "es"
			root.add_child(dialog)
			_language = locale
			var key: String = test["key"]
			var arguments: Array = test["arguments"]
			if key == "R_VERSION_START_ERROR":
				dialog.call("_show_failure", key, arguments)
			else:
				dialog.call("_request_completed", int(test["result"]), int(test["http"]), PackedStringArray(), test["body"])
			var detail: String = expected[locale][key]
			if not arguments.is_empty():
				detail = detail % arguments
			var prefix: String = source[locale]["T_CANT_CHECK_CURRENT_VERSION"]
			var wanted: String = prefix + ("" if prefix.ends_with(" ") else " ") + detail
			var text: String = (dialog.get("_text") as Label).text
			_check(text == wanted, locale + ": exact detail and diagnostics for " + key)
			_check(dialog.title == source[locale]["T_VERSION_TITLE"], locale + ": completion uses current locale for " + key)
			_check(bool(dialog.get("_finished")), locale + ": failure completes once for " + key)
			dialog.call("_request_completed", HTTPRequest.RESULT_SUCCESS, 200, PackedStringArray(), "900".to_utf8_buffer())
			_check((dialog.get("_text") as Label).text == text, locale + ": late response preserves error for " + key)
			var before: int = _closed
			dialog.closed.connect(func() -> void: _closed += 1)
			dialog.close_check()
			dialog.close_check()
			await process_frame
			_check(_closed == before + 1 and not is_instance_valid(dialog), locale + ": idempotent close for " + key)
			_observations.append({"language": locale, "key": key, "text": text})
	print("VERSION_FEEDBACK_RESULT ", JSON.stringify({"status": "PASS" if _failures.is_empty() else "FAIL", "checks": _checks, "cases": _observations.size(), "locales": LOCALES, "actual_transport_claim": false, "failures": _failures}))
	quit(0 if _failures.is_empty() else 1)
