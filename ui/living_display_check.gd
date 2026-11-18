extends SceneTree

const Main = preload("res://ui/main.gd")

class TimedMain extends Main:
	var now_usec: int = 1000000

	func _clock_usec() -> int:
		return now_usec

var _checks: int = 0
var _failures: Array[String] = []


func _initialize() -> void:
	_run.call_deferred()


func _check(condition: bool, message: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(message)


func _run() -> void:
	root.size = Vector2i(1280, 800)
	root.gui_embed_subwindows = true
	var simulation: BgrSimulation = BgrSimulation.new()
	_check(bool(simulation.preferences_initialize()["ok"]), "Initialize the isolated preferences")
	var application: TimedMain = TimedMain.new()
	application.startup_seed_text = "827364"
	application.startup_dimensions = Vector2i(96, 96)
	application.startup_attempts = 0
	_check(application.initialize_session(simulation), "Use the actual Rust session")
	root.add_child(application)
	application.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	application.set_process(false)
	await process_frame
	await process_frame
	var canvas: Control = application.get("_canvas") as Control
	var shell: Control = application.get("_ambient_shell") as Control
	shell.set_process(false)
	_check(canvas.get_global_rect().is_equal_approx(Rect2(Vector2.ZERO, Vector2(1280, 800))), "The world occupies the complete viewport")
	var world_rectangle: Rect2 = canvas.get_global_rect()
	for name: String in ["PauseButton", "CreateWorldButton", "ResetButton", "SeedInput", "WidthInput", "HeightInput", "AttemptsInput", "SaveWorldButton", "BackupWorldButton", "ManageConnectionsButton", "WorldActionsMenu", "GlobalActionsMenu", "WorldToolsMenu", "GeneticsMenu", "SettingsButton", "StatisticsButton", "HelpMenu", "CrispLines", "FullscreenButton"]:
		_check(application.find_child(name, true, false) != null, "Retain original action: " + name)
	shell.call("show_tools")
	await process_frame
	_check((application.find_child("WorldActionsMenu", true, false) as Control).is_visible_in_tree(), "The original menus are reachable in the tools drawer")
	_check(canvas.get_global_rect().is_equal_approx(world_rectangle), "Tools do not resize the ecology")
	for name: String in ["WorldActionsMenu", "GlobalActionsMenu", "WorldToolsMenu", "NetworkMenu", "GeneticsMenu", "HelpMenu"]:
		var menu: MenuButton = application.find_child(name, true, false) as MenuButton
		_check(not menu.flat and menu.size.y >= 48.0, "The menu is shown as a touch-size button: " + name)
		var position: Vector2 = menu.get_global_rect().get_center()
		for pressed: bool in [true, false]:
			var event: InputEventMouseButton = InputEventMouseButton.new()
			event.button_index = MOUSE_BUTTON_LEFT
			event.position = position
			event.global_position = position
			event.pressed = pressed
			root.push_input(event, true)
		await process_frame
		_check(menu.get_popup().visible and menu.get_popup().is_embedded(), "Click opens the menu on the simulation surface: " + name)
		menu.get_popup().hide()
		await process_frame
	shell.call("toggle_tools")
	(application.find_child("AmbientPause", true, false) as Button).pressed.emit()
	_check(not bool(simulation.snapshot()["paused"]), "The floating control resumes the actual world")
	_check(simulation.has_method("living_scene_snapshot"), "Integrate the current native living-scene API")
	for index: int in range(24):
		shell.call("_advance_idle", 0.25)
	var initial_view: Dictionary = canvas.call("view_state")
	var before: Dictionary = simulation.snapshot()
	for index: int in range(40):
		application.call("_update_cinema", 0.1)
	_check(canvas.call("view_state") != initial_view, "Idle automation changes the actual canvas view")
	var cards: Control = application.get("_living_cards") as Control
	_check(bool(cards.get("_available")), "Main supplies actual native records to its cards")
	_check(simulation.snapshot() == before, "Camera presentation does not change the world")
	var view: Dictionary = canvas.call("view_state")
	application.call("_manual_camera_override")
	application.call("_update_cinema", 0.25)
	_check(canvas.call("view_state") == view, "Manual interaction suspends the automatic camera")
	application.set("_manual_camera_until", 0.0)
	application.call("_update_cinema", 60.1)
	_check((application.find_child("LivingCard", true, false) as Control).visible, "Main shows a scheduled card through its real data path")
	cards.call("dismiss")
	cards.set("_next_mode", 3)
	application.call("_update_cinema", 60.1)
	_check(bool(cards.get("_panel").visible) and int(cards.get("_mode")) == 3, "Main schedules an all-time records slide")
	_check(cards.get("_all_time_records").size() == 3 and not bool(cards.call("needs_all_time_records")), "Main supplies all three native historical record slots")
	shell.call("show_tools")
	application.call("_update_cinema", 0.1)
	_check(not (application.find_child("LivingCard", true, false) as Control).visible, "Opening controls dismisses automatic slides")
	_check(cards.get("_all_time_records").is_empty(), "Dismissal releases the historical slide data")
	shell.call("toggle_tools")
	for index: int in range(24):
		shell.call("_advance_idle", 0.25)
	cards.set("slide_interval_seconds", 10.0)
	application.now_usec += 11000000
	application.call("_process", 0.01)
	_check((application.find_child("LivingCard", true, false) as Control).visible, "Use monotonic wall time when engine frame delta differs")
	(application.find_child("AmbientPause", true, false) as Button).pressed.emit()
	_check(bool(simulation.snapshot()["paused"]), "Pause remains an explicit simulation action")
	for language: String in ["en", "es", "ca", "de", "pt", "ru"]:
		shell.call("set_language", language)
		_check(not (application.find_child("AmbientTools", true, false) as Button).text.is_empty(), "Keep a usable controls label for " + language)
	for index: int in range(24):
		shell.call("_advance_idle", 0.25)
	_check(not (application.find_child("AmbientHud", true, false) as Control).visible, "Inactivity removes the complete overlay")
	_check(canvas.get_global_rect().is_equal_approx(world_rectangle) and canvas.modulate == Color.WHITE, "Fading leaves the full world unchanged")
	application.call("_show_error", "Owned display failure")
	_check((application.find_child("AmbientToolsDrawer", true, false) as Control).visible, "Actionable failures reveal the retained feedback")
	application.queue_free()
	await process_frame
	print("BGR_LIVING_DISPLAY " + JSON.stringify({"checks": _checks, "failures": _failures, "result": "PASS" if _failures.is_empty() else "FAIL", "physical_fullscreen_or_touch": false}))
	quit(0 if _failures.is_empty() else 1)
