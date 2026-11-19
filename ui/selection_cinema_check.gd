extends SceneTree

const Main = preload("res://ui/main.gd")

var failures: Array[String] = []
var checks: int = 0

func _initialize() -> void:
	_run.call_deferred()

func check(value: bool, message: String) -> void:
	checks += 1
	if not value:
		failures.append(message)

func click(point: Vector2) -> void:
	for down: bool in [true, false]:
		var event: InputEventMouseButton = InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
		event.position = point
		event.global_position = point
		event.pressed = down
		root.push_input(event, true)
		await process_frame

func touch(index: int, down: bool, point: Vector2) -> void:
	var event: InputEventScreenTouch = InputEventScreenTouch.new()
	event.index = index
	event.position = point
	event.pressed = down
	root.push_input(event, true)
	await process_frame

func drag(index: int, point: Vector2, relative: Vector2) -> void:
	var event: InputEventScreenDrag = InputEventScreenDrag.new()
	event.index = index
	event.position = point
	event.relative = relative
	root.push_input(event, true)
	await process_frame

func _run() -> void:
	root.size = Vector2i(1280, 800)
	root.gui_embed_subwindows = true
	var simulation: BgrSimulation = BgrSimulation.new()
	check(bool(simulation.preferences_initialize()["ok"]), "Initialize isolated preferences")
	var main: Main = Main.new()
	main.startup_seed_text = "827364"
	main.startup_dimensions = Vector2i(7000, 7000)
	main.startup_attempts = 12
	check(main.initialize_session(simulation), "Initialize real world")
	root.add_child(main)
	main.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	main.set_process(false)
	await process_frame
	await process_frame
	var shell: Control = main.get("_ambient_shell") as Control
	shell.set_process(false)
	var canvas: Control = main.get("_canvas") as Control
	var cards: Control = main.get("_living_cards") as Control
	var lines: PackedFloat64Array = simulation.snapshot()["segments"]
	check(lines.size() >= 4, "Fixture contains an organism")
	if lines.size() < 4:
		print(JSON.stringify({"checks": checks, "failures": failures}))
		quit(1)
		return
	var world_point := Vector2((lines[0] + lines[2]) * 0.5, (lines[1] + lines[3]) * 0.5)
	var point: Vector2 = canvas.global_position + (canvas.get("_offset") as Vector2) + world_point * float(canvas.get("_zoom"))
	await click(point)
	check(not simulation.snapshot()["selected"].is_empty(), "Canvas click selects an organism")
	var inspector: Control = main.find_child("AmbientInspector", true, false) as Control
	check(inspector.is_visible_in_tree(), "Selection shows inspector")
	shell.call("show_tools")
	await process_frame
	check(inspector.is_visible_in_tree(), "Tools leave inspector visible")
	shell.call("toggle_tools")
	if bool(simulation.snapshot()["paused"]):
		(main.find_child("AmbientPause", true, false) as Button).pressed.emit()
	check(not bool(simulation.snapshot()["paused"]), "Run the world before checking cinema")
	main.set("_cinema_enabled", true)
	main.set("_manual_camera_until", 0.0)
	for index: int in range(24):
		shell.call("_advance_idle", 0.25)
	check(bool(shell.call("is_idle")) and inspector.is_visible_in_tree(), "Inspector remains visible after HUD fade")
	check(bool(main.call("_cinema_allowed")), "Selected organism does not block cinema")
	main.call("_update_cinema", 0.1)
	main.call("_update_cinema", 30.1)
	await process_frame
	await process_frame
	var card: Control = main.find_child("LivingCard", true, false) as Control
	check(card.is_visible_in_tree(), "Cinema shows a real card with an active selection")
	var card_rect := Rect2(card.global_position, card.size * card.scale)
	check(not card_rect.intersects(inspector.get_global_rect()), "Cinema card avoids inspector")
	var capture: String = OS.get_environment("BGR_SELECTION_CINEMA_CAPTURE")
	if not capture.is_empty():
		await RenderingServer.frame_post_draw
		check(root.get_texture().get_image().save_png(capture) == OK, "Save selection and cinema layout capture")
	var kill: Button = main.find_child("KillOrganismButton", true, false) as Button
	(shell.get("_inspector_scroll") as ScrollContainer).ensure_control_visible(kill)
	await process_frame
	await click(kill.get_global_rect().get_center())
	check(not simulation.snapshot()["selected"].is_empty() and not bool(simulation.snapshot()["selected"]["alive"]), "Inspector kill action retains a dead selection")
	for index: int in range(24):
		shell.call("_advance_idle", 0.25)
	check(bool(shell.call("is_idle")) and inspector.is_visible_in_tree() and bool(main.call("_cinema_allowed")), "Dead selection remains visible while cinema is eligible")
	var hud: Control = main.find_child("AmbientHud", true, false) as Control
	await touch(0, true, Vector2(620, 750))
	await touch(1, true, Vector2(660, 750))
	await drag(1, Vector2(680, 750), Vector2(20, 0))
	check(not hud.visible and (canvas.get("_touches") as Dictionary).size() == 2, "Two touches and a drag reach the canvas beneath the hidden dock")
	await touch(0, false, Vector2(620, 750))
	check(not hud.visible and (canvas.get("_touches") as Dictionary).size() == 1, "First release keeps the HUD hidden and the second contact active")
	await touch(1, false, Vector2(680, 750))
	check(hud.visible and (canvas.get("_touches") as Dictionary).is_empty(), "Final release reaches the canvas before the HUD returns")
	for index: int in range(24):
		shell.call("_advance_idle", 0.25)
	await click(Vector2(640, 750))
	check(simulation.snapshot()["selected"].is_empty(), "First idle blank canvas click beneath the hidden dock deselects")
	check(not inspector.is_visible_in_tree(), "Deselection hides inspector")
	main.call("_update_cinema", 0.1)
	check((cards.get("_avoid_rect") as Rect2) == Rect2(), "Main clears the card avoidance rectangle")
	main.queue_free()
	await process_frame
	print("BGR_SELECTION_CINEMA " + JSON.stringify({"checks": checks, "failures": failures}))
	quit(0 if failures.is_empty() else 1)
