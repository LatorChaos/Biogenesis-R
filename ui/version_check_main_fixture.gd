extends "res://ui/main.gd"

const FixtureVersion = preload("res://ui/version_check_fixture.gd")

var qa_case_path: String = "/duplicate"


func _new_version_check() -> VersionCheck:
	var dialog: FixtureVersion = FixtureVersion.new()
	dialog.case_path = qa_case_path
	return dialog
