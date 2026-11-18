extends "res://ui/version_dialog.gd"

var case_path: String = "/line-crlf"


func _request_url() -> String:
	var base: String = OS.get_environment("BGR_VERSION_TEST_BASE_URL")
	if case_path == "/unavailable":
		return "http://127.0.0.1:" + OS.get_environment("BGR_VERSION_TEST_UNAVAILABLE_PORT") + "/unavailable"
	return base + case_path
