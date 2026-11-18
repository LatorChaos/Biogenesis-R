extends SceneTree

const Cards = preload("res://ui/living_cards.gd")

class RefusingPreview:
	extends "res://ui/genome_preview.gd"
	var refuse: bool = false
	var attempts: int = 0

	func set_raster(image: Dictionary, dimensions: Vector2i = Vector2i(100, 100)) -> bool:
		if not image.is_empty():
			attempts += 1
			if refuse:
				return false
		return super.set_raster(image, dimensions)


var _checks: int = 0
var _failures: Array[String] = []
var _focused: Array[int] = []


func _initialize() -> void:
	_run.call_deferred()


func _check(condition: bool, description: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(description)


func _actor(token: int, identifier: int, children: int, kills: int, age: int, mass: float, infections: int) -> Dictionary:
	return {"token": token, "id": identifier, "alive": true, "center": PackedFloat64Array([1.0, 2.0]), "bounds": PackedInt32Array([0, 0, 10, 10]), "age_ticks": age, "age_world_units": age >> 8, "mass": mass, "children": children, "kills": kills, "infections": infections}


func _record(kind: String, unit: String, token: int, value: Variant) -> Dictionary:
	return {"kind": kind, "unit": unit, "present": token != 0, "subject_token": token, "value": value}


func _packet() -> Dictionary:
	return {"schema": 1, "ok": true, "world_ready": true, "presentation_epoch": 1, "subjects": [
		_actor(11, 7, 2, 0, 513, 12.0, 0), _actor(12, 8, 0, 3, 256, 18.0, 4)], "records": [
		_record("offspring", "count", 11, 2), _record("kills", "count", 12, 3),
		_record("age", "biological_ticks", 11, 513), _record("mass", "source_mass", 12, 18.0),
		_record("infections", "count", 12, 4)]}


func _empty() -> Dictionary:
	var packet: Dictionary = _packet()
	packet["subjects"] = []
	for record: Dictionary in packet["records"]:
		record["present"] = false
		record["subject_token"] = 0
		record["value"] = 0.0 if record["kind"] == "mass" else 0
	return packet


func _context(language: String = "en") -> Dictionary:
	return {"population": 2, "world_time": 17, "language": language, "gases": {"oxygen": 120.5, "carbon_dioxide": 80.25, "methane": 0.0}}


func _show(cards: Cards) -> void:
	cards.advance(cards.slide_interval_seconds, true)


func _run() -> void:
	root.size = Vector2i(1280, 800)
	var cards: Cards = Cards.new()
	root.add_child(cards)
	cards.focus_subject.connect(func(token: int) -> void: _focused.append(token))
	await process_frame
	_check(cards.set_scene(_packet(), _context()), "Accept the exact living-record packet")
	_check(not cards._panel.visible, "Begin with an unobstructed world")
	cards.advance(59.0, true)
	_check(not cards._panel.visible, "Keep a long bare-world interval")
	cards.advance(1.0, true)
	await process_frame
	_check(cards._panel.visible and cards._number.text == "2" and cards._focus_token == 11, "Show the actual offspring champion after sixty seconds")
	_check(cards._panel.size.x <= 360.0 and cards._panel.size.y <= 300.0, "Keep the card within the display footprint")
	_check(cards._panel.size.x < 300.0 and cards._panel.size.y < 200.0, "Fit a portrait-free record around its title and statistics")
	_check(cards.mouse_filter == Control.MOUSE_FILTER_IGNORE and cards._panel.mouse_filter == Control.MOUSE_FILTER_IGNORE and cards._body.mouse_filter == Control.MOUSE_FILTER_IGNORE, "Keep world input transparent")
	cards.advance(8.0, true)
	_check(not cards._panel.visible, "Dismiss after the declared dwell")
	_show(cards)
	_check(cards._mode == 1 and cards._leader_labels.size() == 5 and cards._leaders.visible, "List all five source-defined living categories")
	_check(cards._leader_labels[1].text == "Victims\n3 #8", "Retain the source kill count, label and organism ID")
	_check(cards._leader_labels[2].text == "Age\n2 #7\n256 age ticks" and cards._leader_labels[3].text == "Mass\n18.00 #8\nSource mass", "Retain age and mass values with source units")
	cards.dismiss()
	_show(cards)
	_check(cards._mode == 2 and cards._number.text == "2" and cards._body.text.contains("120.50") and cards._body.text.contains("0.00"), "Show actual population and gases including zero")
	cards._input(InputEventMouseMotion.new())
	_check(not cards._panel.visible and cards._elapsed == 0.0, "Mouse activity dismisses and resets cadence")
	_show(cards)
	cards._input(InputEventScreenTouch.new())
	_check(not cards._panel.visible, "Touch activity dismisses")
	_show(cards)
	cards._input(InputEventKey.new())
	_check(not cards._panel.visible, "Keyboard activity dismisses")
	cards.advance(30.0, true)
	cards.advance(40.0, false)
	_check(not cards._panel.visible and cards._elapsed == 0.0, "Modal fences discard pending slide time")
	cards.advance(59.0, true)
	_check(not cards._panel.visible, "Require a fresh interval after a modal fence")
	cards.advance(NAN, true)
	_check(not cards._panel.visible, "Ignore nonfinite elapsed time")
	for language: String in ["en", "es", "ca", "de", "pt", "ru"]:
		cards.set_scene(_packet(), _context(language))
		cards.dismiss()
		cards._next_mode = 0
		cards._next_metric = 2
		_show(cards)
		await process_frame
		_check(cards._number.text == "2" and cards._body.text.contains(str(Cards.WORDS[language][6])) and not cards._title.text.contains("!"), "Localize source age units: " + language)
		cards.dismiss()
		cards._next_mode = 1
		_show(cards)
		await process_frame
		_check(cards._panel.size.x > 360.0 and cards._panel.size.x <= 720.0 and cards._panel.size.y <= 264.0, "Widen only the leader card within its display footprint: " + language)
	cards.set_scene(_packet(), _context())
	cards.dismiss()
	cards._next_mode = 0
	cards._next_metric = 0
	_show(cards)
	cards.focus_current_subject()
	_check(_focused == [11] and not cards._panel.visible, "Emit only the current live focus token through an explicit action")
	var changed: Dictionary = _packet()
	changed["presentation_epoch"] = 2
	cards.set_scene(changed, _context())
	_check(not cards._panel.visible and cards._elapsed == 0.0, "Reset presentation cadence on a new epoch")
	_show(cards)
	changed["subjects"].remove_at(0)
	changed["records"][0] = _record("offspring", "count", 0, 0)
	changed["records"][2] = _record("age", "biological_ticks", 12, 256)
	_check(cards.set_scene(changed, _context()) and not cards._panel.visible, "Dismiss when the shown actor leaves the current live packet")
	cards.focus_current_subject()
	_check(_focused == [11], "Never focus a removed token")
	var empty: Dictionary = _empty()
	_check(cards.set_scene(empty, {"population": 0, "world_time": 0}), "Accept an empty live population")
	cards._next_mode = 0
	_show(cards)
	_check(cards._number.text == "0" and cards._body.text == Cards.WORDS["en"][3], "Distinguish no living organisms from an unavailable packet")
	var zero: Dictionary = _empty()
	zero["subjects"] = [_actor(15, 9, 0, 0, 0, 0.0, 0)]
	zero["records"][2] = _record("age", "biological_ticks", 15, 0)
	zero["records"][3] = _record("mass", "source_mass", 15, 0.0)
	cards.dismiss()
	cards._next_mode = 0
	cards._next_metric = 0
	_check(cards.set_scene(zero, _context()), "Accept genuine zero-valued live actors")
	_show(cards)
	_check(cards._number.text == "0" and cards._body.text == Cards.WORDS["en"][4], "Distinguish no positive counter record from no living organisms")
	for mutation: String in ["dead", "missing", "duplicate", "metric", "unit", "value", "nonfinite", "age", "over_limit", "missing_age", "boolean"]:
		var bad: Dictionary = _packet()
		match mutation:
			"dead": bad["subjects"][0]["alive"] = false
			"missing": bad.erase("subjects")
			"duplicate": bad["subjects"].append(bad["subjects"][0])
			"metric": bad["records"][0]["kind"] = "predation"
			"unit": bad["records"][2]["unit"] = "seconds"
			"value": bad["records"][0]["value"] = 90
			"nonfinite": bad["subjects"][0]["mass"] = INF
			"age": bad["subjects"][0]["age_world_units"] = 513
			"over_limit":
				for index: int in range(7):
					bad["subjects"].append(_actor(100 + index, index, 0, 0, 0, 0.0, 0))
			"missing_age": bad["records"][2] = _record("age", "biological_ticks", 0, 0)
			"boolean": bad["subjects"][0]["alive"] = 1
		_check(not cards.set_scene(bad, _context()) and not cards._panel.visible, "Reject invalid current data: " + mutation)
	_check(not cards.set_scene({"ok": false, "error": "unavailable"}, _context()) and not cards._available, "Clear unavailable data without fabricating zero records")
	cards.advance(3600.0, true)
	_check(not cards._panel.visible, "Unavailable data never produces a slide")
	await _preview_checks(cards)
	await _all_time_checks(cards)
	cards.free()
	print(JSON.stringify({"checks": _checks, "failures": _failures, "simulation_calls": 0}))
	quit(0 if _failures.is_empty() else 1)


func _raster(red: int) -> Dictionary:
	var pixels: PackedByteArray = PackedByteArray()
	pixels.resize(30000)
	for index: int in range(0, pixels.size(), 3):
		pixels[index] = red
	return {"width": 100, "height": 100, "rgb": pixels}


func _with_previews() -> Dictionary:
	var packet: Dictionary = _packet()
	for actor: Dictionary in packet["subjects"]:
		actor["preview"] = {"requested": true, "error": "", "image": _raster(64 if actor["token"] == 11 else 192)}
	return packet


func _preview_checks(cards: Cards) -> void:
	cards.set_scene(_packet(), _context())
	cards.dismiss()
	cards._next_mode = 0
	cards._next_metric = 0
	_check(not cards.needs_previews(), "Do not request native images during the bare-world interval")
	_show(cards)
	_check(cards.needs_previews(), "Request the current live record image when its slide appears")
	_check(cards.set_scene(_with_previews(), _context()), "Accept bounded native RGB previews")
	await process_frame
	_check(not cards.needs_previews() and cards._preview._raster.get_image().get_data() == _raster(64)["rgb"], "Show the exact matching record raster beside its ID and value")
	_check(cards._preview.get_parent().visible and cards._preview.mouse_filter == Control.MOUSE_FILTER_IGNORE, "A passive preview cannot capture world input")
	var texture: ImageTexture = cards._preview._raster
	cards.set_scene(_packet(), _context())
	_check(not cards.needs_previews() and cards._preview._raster == texture, "Ordinary polls retain a slide image without native regeneration")
	_check(cards._panel.size.x <= 360.0 and cards._panel.size.y <= 300.0 and cards._preview.get_global_rect().end.x < cards._number.get_global_rect().position.x, "Place the record appearance beside statistics within the existing footprint")
	_check(cards._preview.size == Vector2(100.0, 100.0) and cards._panel.size.x < 300.0 and cards._panel.size.y < 200.0, "Fit a record tightly while keeping its native portrait size")
	cards.dismiss()
	_check(not cards.needs_previews() and cards._preview_images.is_empty(), "Activity clears the slide cache and request")
	cards._next_mode = 1
	_show(cards)
	cards.set_scene(_with_previews(), _context())
	await process_frame
	for index: int in range(5):
		var red: int = 64 if index in [0, 2] else 192
		_check(cards._leader_previews[index]._raster.get_image().get_data() == _raster(red)["rgb"] and cards._leader_labels[index].text.contains("#7" if red == 64 else "#8"), "Bind each leader appearance to its source token: " + str(index))
	for language: String in ["en", "es", "ca", "de", "pt", "ru"]:
		cards.set_scene(_with_previews(), _context(language))
		await process_frame
		var inner: Rect2 = cards._panel.get_global_rect().grow(-15.0)
		for index: int in range(5):
			var portrait: Rect2 = cards._leader_previews[index].get_global_rect()
			var label: Rect2 = cards._leader_labels[index].get_global_rect()
			_check(portrait.size == Vector2(100.0, 100.0) and cards._leader_previews[index].scale == Vector2.ONE and inner.encloses(portrait) and inner.encloses(label), "Fit each native-size portrait and localized label: %s %d, portrait %s, label %s, inner %s" % [language, index, portrait, label, inner])
	_check(cards._panel.size.x < 690.0 and cards._panel.size.y < 264.0, "Fit the living leaderboard around its five portrait columns")
	await _leader_resize_checks(cards)
	cards._input(InputEventScreenTouch.new())
	_check(not cards._panel.visible and cards._preview_images.is_empty(), "Touch dismisses the larger leader card and clears its cache")
	var missing: Dictionary = _packet()
	missing["subjects"][1]["preview"] = {"requested": true, "error": "Primary genotype unavailable", "image": {}}
	cards.dismiss()
	cards._next_mode = 0
	cards._next_metric = 3
	cards.set_scene(missing, _context())
	_show(cards)
	cards.set_scene(missing, _context())
	_check(not cards.needs_previews() and not cards._preview.get_parent().visible and cards._body.text.contains("#8"), "An unavailable appearance keeps truthful statistics without fake art or repeated requests")
	var bad: Dictionary = _with_previews()
	bad["subjects"][0]["preview"]["image"]["rgb"] = PackedByteArray([1])
	_check(not cards.set_scene(bad, _context()) and not cards._panel.visible, "Reject malformed raster sizes before publication")
	cards.set_scene(_packet(), _context())
	cards._next_mode = 0
	cards._next_metric = 0
	_show(cards)
	cards.set_scene(_with_previews(), _context())
	var reset: Dictionary = _packet()
	reset["presentation_epoch"] = 2
	cards.set_scene(reset, _context())
	_check(not cards._panel.visible and cards._preview_images.is_empty(), "Epoch replacement removes all old appearances")
	cards._next_mode = 2
	_show(cards)
	_check(not cards.needs_previews() and not cards._preview.get_parent().visible, "Ecology facts never borrow an unrelated organism picture")
	var holder: Control = Control.new()
	cards.add_child(holder)
	var refusing: RefusingPreview = RefusingPreview.new()
	holder.add_child(refusing)
	cards._set_preview(refusing, _raster(64))
	_check(holder.visible and refusing._raster != null, "Establish a previous successful organism raster")
	refusing.refuse = true
	cards._set_preview(refusing, _raster(192))
	_check(not holder.visible and refusing._raster == null, "A failed new image cannot show the previous organism")
	var attempts: int = refusing.attempts
	cards._set_preview(refusing, _raster(192))
	_check(refusing.attempts == attempts and not holder.visible, "Failed image publication does not retry on ordinary polls")
	holder.free()


func _all_time_packet() -> Dictionary:
	var packet: Dictionary = _empty()
	packet["all_time_records"] = [
		{"kind": "offspring", "count": 17, "time": 12, "present": true, "preview": {"requested": true, "error": "", "image": _raster(124)}},
		{"kind": "kills", "count": 0, "time": 0, "present": false, "preview": {"requested": false, "error": "", "image": {}}},
		{"kind": "infections", "count": -3, "time": -55, "present": true, "preview": {"requested": true, "error": "Geometry unavailable", "image": {}}}
	]
	return packet


func _leader_resize_checks(cards: Cards) -> void:
	var packet: Dictionary = _with_previews()
	for actor: Dictionary in packet["subjects"]:
		actor["id"] = 9223372036854775807
	cards.set_scene(packet, _context("ru"))
	await process_frame
	await process_frame
	var full_width: float = cards._panel.size.x
	root.size = Vector2i(700, 660)
	await process_frame
	await process_frame
	var panel: Rect2 = cards._panel.get_global_rect()
	_check(panel.size.x < full_width and is_equal_approx(panel.end.x, cards.size.x - 20.0), "Reflow the living card and keep it aligned to the upper-right edge")
	for index: int in range(5):
		var label: Label = cards._leader_labels[index]
		_check(panel.grow(-15.0).encloses(label.get_global_rect()) and label.get_visible_line_count() == label.get_line_count() and cards._leader_previews[index].size == Vector2(100.0, 100.0), "Keep all large identifier text and native portrait sizes after reflow: " + str(index))
	root.size = Vector2i(1280, 800)
	cards.set_scene(_with_previews(), _context("ru"))
	await process_frame
	await process_frame
	_check(cards._panel.size.x < full_width and is_equal_approx(cards._panel.get_global_rect().end.x, cards.size.x - 20.0), "Shrink the card after shorter statistics arrive and the viewport grows")


func _all_time_checks(cards: Cards) -> void:
	cards.dismiss()
	var packet: Dictionary = _all_time_packet()
	cards.set_scene(packet, _context())
	_check(not cards.needs_all_time_records() and cards._all_time_records.is_empty(), "Ignore all-time data outside its scheduled slide")
	cards.set_scene(_packet(), _context())
	cards._next_mode = 1
	_show(cards)
	cards.set_scene(_with_previews(), _context())
	_check(cards._leader_previews[0]._raster != null, "Initialize the shared portrait with a living preview")
	cards.advance(cards.slide_dwell_seconds, true)
	_check(cards._leader_previews[0]._raster == null, "Dismiss the living preview before the shared portrait slot is reused")
	cards._next_mode = 2
	_show(cards)
	_check(not cards.needs_all_time_records(), "An ecology slide makes no all-time request")
	cards.advance(cards.slide_dwell_seconds, true)
	_show(cards)
	_check(cards.needs_all_time_records() and not cards.needs_previews(), "The fourth slide requests one all-time packet without living previews")
	_check(cards.set_scene(packet, _context()), "Accept historic records with no living actors")
	await process_frame
	_check(not cards.needs_all_time_records() and cards._leaders.visible and cards._mode == 3, "Publish the all-time snapshot after receiving its data")
	_check(cards._title.text == "All-time records - Living and dead", "Identify the all-time scope")
	_check(cards._leader_labels[0].text == "Children\n17\nTime: 12" and cards._leader_labels[1].text == "Victims\n0\nTime: 0" and cards._leader_labels[2].text == "Infected\n-3\nTime: -55", "Show three source categories, their signed counts and their recorded times")
	_check(cards._leader_previews[0]._raster.get_image().get_data() == _raster(124)["rgb"], "Render the exact retained historical genome raster")
	_check(not cards._leader_previews[1].visible and not cards._leader_previews[2].visible and cards._appearance_unavailable[1].visible and cards._appearance_unavailable[2].visible, "Show unavailable appearances without inventing imagery")
	_check(not cards._leader_labels[3].is_visible_in_tree() and not cards._leader_labels[4].is_visible_in_tree(), "Show only the categories with source-defined all-time records")
	_check(cards._panel.size.x < 465.0 and cards._panel.size.y < 264.0, "Fit the all-time leaderboard around its three portrait columns")
	cards.focus_current_subject()
	_check(cards._panel.visible and cards.current_subject_token() == 0 and _focused == [11], "An all-time record cannot become a living follow target")
	var texture: ImageTexture = cards._leader_previews[0]._raster
	for language: String in ["en", "es", "ca", "de", "pt", "ru"]:
		cards.set_scene(_empty(), _context(language))
		await process_frame
		_check(not cards.needs_all_time_records() and cards._leader_previews[0]._raster == texture, "Ordinary living polls retain the all-time snapshot: " + language)
		var inner: Rect2 = cards._panel.get_global_rect().grow(-15.0)
		var scope: String = str(Cards.WORDS[language][10])
		_check(cards._title.text.contains(scope) and cards._appearance_unavailable[1].text == str(Cards.WORDS[language][11]) and inner.encloses(cards._title.get_global_rect()) and inner.encloses(cards._leader_labels[2].get_global_rect()), "Fit the localized all-time card: " + language)
	packet["presentation_epoch"] = 2
	_check(cards.set_scene(packet, _context()) and not cards._panel.visible and cards._all_time_records.is_empty() and cards._leader_previews[0]._raster == null, "A new world drops the visible historical snapshot and its raster")
	cards._next_mode = 3
	_show(cards)
	_check(cards.needs_all_time_records() and not cards._leaders.visible, "Await a fresh historical snapshot on the new world")
	packet["all_time_records"][0]["preview"]["image"]["rgb"] = PackedByteArray([1])
	_check(not cards.set_scene(packet, _context()) and not cards._panel.visible, "Reject an invalid historical raster before publication")
