extends SceneTree

const HelpAboutScript = preload("res://ui/help_about.gd")
const LOCALES: Array[String] = ["en", "es", "ca", "de", "pt", "ru"]

var _checks: int = 0
var _failures: Array[String] = []


func _initialize() -> void:
	_run.call_deferred()


func _run() -> void:
	var run: String = OS.get_environment("BGR_TEST_ROOT")
	if not run.is_absolute_path() or not DirAccess.dir_exists_absolute(run):
		push_error("BGR_TEST_ROOT must name an existing owned absolute directory")
		quit(2)
		return
	var fixture: String = run.path_join("manual_selector")
	if FileAccess.file_exists(fixture) or DirAccess.dir_exists_absolute(fixture):
		push_error("Preserve the prior manual selector fixture")
		quit(2)
		return
	var expected: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://ui/help_about_expected.json"))
	var locales: Dictionary = expected["locales"]
	for locale: String in LOCALES:
		var messages: Dictionary = locales[locale]
		for key: String in messages:
			_check(HelpAboutScript.message(key, locale) == str(messages[key]), locale + ":" + key)
		_check(HelpAboutScript.about_text(locale) == str(expected["about"][locale]), locale + ": exact About text")
	for name: String in ["empty", "english", "localized", "localized_directory/manual_ca.html", "english_directory/manual_en.html"]:
		if DirAccess.make_dir_recursive_absolute(fixture.path_join(name)) != OK:
			push_error("Cannot create owned selector fixture")
			quit(2)
			return
	if not _write_fixture(fixture.path_join("english/manual_en.html"), "Owned English manual fixture"):
		quit(2)
		return
	for locale: String in LOCALES:
		if not _write_fixture(fixture.path_join("localized/manual_" + locale + ".html"), "Owned " + locale + " manual fixture"):
			quit(2)
			return
	for locale: String in LOCALES:
		_check(HelpAboutScript.manual_target(locale, fixture.path_join("empty")) == str(expected["manual_urls"][locale]), locale + ": Source remote fallback")
		_check(HelpAboutScript.manual_target(locale, fixture.path_join("english")) == fixture.path_join("english/manual_en.html"), locale + ": installed English fallback")
		_check(HelpAboutScript.manual_target(locale, fixture.path_join("localized")) == fixture.path_join("localized/manual_" + locale + ".html"), locale + ": localized existing path wins")
	_check(HelpAboutScript.manual_target("ca", fixture.path_join("localized_directory")) == fixture.path_join("localized_directory/manual_ca.html"), "Source exists accepts localized directory")
	_check(HelpAboutScript.manual_target("de", fixture.path_join("english_directory")) == fixture.path_join("english_directory/manual_en.html"), "Source exists accepts English directory")
	var artwork: PackedByteArray = FileAccess.get_file_as_bytes("res://ui/assets/bullet.jpg")
	var hash: HashingContext = HashingContext.new()
	var hashed: bool = hash.start(HashingContext.HASH_SHA256) == OK and hash.update(artwork) == OK
	_check(hashed and hash.finish().hex_encode() == "d4fd46c79d8fac97d8a41ccf9806add5ddbf9fe6269449c383b0bcef5b7cd164" and HelpAboutScript.ARTWORK.get_width() == 30 and HelpAboutScript.ARTWORK.get_height() == 30, "Exact Source artwork bytes and imported dimensions")
	print("HELP_ABOUT_BOUNDARY_RESULT " + JSON.stringify({"status": "PASS" if _failures.is_empty() else "FAIL", "checks": _checks, "source_values": 72, "about_texts": 6, "selector_cases": 20, "artwork_checks": 1, "failures": _failures, "browser_dispatches": 0}))
	quit(0 if _failures.is_empty() else 1)


func _write_fixture(path: String, text: String) -> bool:
	var file: FileAccess = FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		push_error("Cannot write owned manual selector fixture")
		return false
	file.store_string(text)
	file.close()
	return true


func _check(value: bool, description: String) -> void:
	_checks += 1
	if not value:
		_failures.append(description)
