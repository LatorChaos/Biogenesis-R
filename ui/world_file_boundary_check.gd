extends SceneTree

const Main = preload("res://ui/main.gd")

var _application: Control
var _checks: int = 0
var _close_events: int = 0
var _cancel_events: int = 0


func _initialize() -> void:
	await process_frame
	var directory: String = OS.get_environment("BGR_WORLD_FILE_OUTPUT")
	if not _check(directory.is_absolute_path() and DirAccess.dir_exists_absolute(directory), "An existing private absolute output directory is required"):
		return
	var path: String = directory.path_join("world-ui-scenario.bgw")
	var other: String = directory.path_join("world-ui-other.bgw")
	var bad: String = directory.path_join("world-ui-invalid.bgw")
	if not _check(not FileAccess.file_exists(path) and not FileAccess.file_exists(other) and not FileAccess.file_exists(bad) and not FileAccess.file_exists(directory.path_join("absent.bgw")) and not DirAccess.dir_exists_absolute(directory.path_join("missing-parent")), "Scenario paths must not exist before this check"):
		return
	var owned_profile: String = OS.get_environment("BGR_TEST_ROOT")
	if not _check(owned_profile.is_absolute_path() and OS.get_user_data_dir().begins_with(owned_profile + "/"), "An owned user-data profile is required"):
		return
	var preferences_path: String = OS.get_user_data_dir().path_join("biogenesis-r.preferences.xml")
	if not _check(not FileAccess.file_exists(preferences_path) and not DirAccess.dir_exists_absolute(preferences_path), "The preference failure scenario needs a fresh profile"):
		return
	var simulation: BgrSimulation = BgrSimulation.new()
	if not _check(simulation.has_method("world_destination") and simulation.has_method("save_world_file") and simulation.has_method("open_world_file"), "World file methods must be registered by the current native library"):
		return
	if not _check(bool(simulation.preferences_initialize()["ok"]), "Owned preferences must initialize before the actual session"):
		return
	_application = Main.new()
	if not _check(bool(_application.call("initialize_session", simulation)), "The application must accept the initialized session"):
		return
	_application.set("startup_seed_text", "827364")
	_application.set("startup_dimensions", Vector2i(1200, 800))
	_application.set("startup_attempts", 32)
	_application.set_process(false)
	root.add_child(_application)
	_application.set_process(false)
	if not _check(not _application.is_processing(), "The fixture must disable Main processing while startup settles"):
		return
	var startup_model: Dictionary = simulation.snapshot()
	await process_frame
	if not bool(simulation.snapshot()["paused"]):
		_application.call("_toggle_pause", "world-file fixture startup")
	_application.set_process(true)
	if not _check(bool(_application.get("_cinema_enabled")), "Default Cinema must remain enabled during fixture startup settlement"):
		return
	var initial: Dictionary = simulation.snapshot()
	if not _check(bool(initial["ok"]), "The actual application entry point must initialize"):
		return
	if not _check(bool(initial["paused"]) and int(initial["tick"]) == 0 and _same_model(startup_model, initial), "The fixture must settle startup and begin paused without advancing the World"):
		return

	_application.call("_toggle_pause")
	_application.call("_on_world_action", 12)
	if not _check(str(_application.get("_file_operation")) == "world_save" and bool(simulation.snapshot()["paused"]), "Unnamed Save must select a destination while paused"):
		return
	_application.call("_finish_file_operation")
	if not _check(str(_application.get("_world_path")).is_empty() and not bool(simulation.snapshot()["paused"]), "Save cancellation must retain the unnamed world and restore running"):
		return

	_application.call("_request_world_decision", "quit")
	_application.call("_world_decision_action", &"save")
	if not _check(bool(_application.get("_awaiting_world_save")), "Quit Save must wait for the actual file choice"):
		return
	_application.call("_finish_file_operation")
	if not _check(not bool(_application.get("_shutting_down")) and str(_application.get("_decision_operation")).is_empty() and not bool(simulation.snapshot()["paused"]), "Cancelling Quit Save must retain the running application"):
		return
	_application.call("_toggle_pause")

	_application.call("_on_world_action", 12)
	_application.call("_on_genotype_file_selected", path.trim_suffix(".bgw"))
	if not _check(str(_application.get("_world_path")) == path and FileAccess.file_exists(path), "Successful unnamed Save must append bgw and own the selected filename"):
		return
	var saved: PackedByteArray = FileAccess.get_file_as_bytes(path)
	if not _check(saved.size() > 4 and saved.slice(0, 4) == PackedByteArray([172, 237, 0, 5]), "The actual saved file must use Java object-stream framing"):
		return
	if not _check(_same_model(initial, simulation.snapshot()), "Saving must preserve the actual world"):
		return

	_application.call("_on_world_action", 13)
	_application.call("_on_genotype_file_selected", path)
	var overwrite: ConfirmationDialog = _application.get("_overwrite_dialog") as ConfirmationDialog
	if not _check(overwrite.visible and str(_application.get("_file_operation")) == "world_save_as", "Save As must require confirmation before replacing an existing file"):
		return
	_application.call("_finish_file_operation")
	if not _check(str(_application.get("_world_path")) == path and FileAccess.get_file_as_bytes(path) == saved, "Overwrite cancellation must retain filename and bytes"):
		return

	_application.call("_on_world_action", 13)
	_application.call("_on_genotype_file_selected", directory.path_join("missing-parent/world.bgw"))
	if not _check(str(_application.get("_file_operation")).is_empty() and str(_application.get("_world_path")) == path and (_application.get("_error_label") as Label).visible, "Failed Save As must release its pause and retain the prior filename"):
		return
	if not _check(not bool(simulation.save_world_file(directory)["ok"]) and FileAccess.get_file_as_bytes(path) == saved, "Invalid destinations must not change prior save bytes"):
		return

	_application.call("_open_statistics")
	_application.call("_open_laboratory", "empty")
	var laboratories: int = _laboratories()
	var invalid: FileAccess = FileAccess.open(bad, FileAccess.WRITE)
	if not _check(invalid != null, "The malformed-file scenario must be writable"):
		return
	invalid.store_buffer(PackedByteArray([0, 1, 2, 3]))
	invalid.close()
	for rejected: String in [directory.path_join("absent.bgw"), bad]:
		if not is_instance_valid(_application.get("_statistics_window")):
			_application.call("_open_statistics")
		_application.call("_on_world_action", 11)
		_application.call("_on_genotype_file_selected", rejected)
		if not _check(str(_application.get("_world_path")) == path and _same_model(initial, simulation.snapshot()) and _laboratories() == laboratories and not is_instance_valid(_application.get("_statistics_window")) and not bool(simulation.snapshot()["statistics_open"]), "Approved failed Open must close statistics and retain the world, filename, and laboratory"):
			return

	_application.call("_single_step")
	var changed: Dictionary = simulation.snapshot()
	_application.call("_on_world_action", 12)
	if not _check(str(_application.get("_file_operation")).is_empty() and str(_application.get("_world_path")) == path and _same_model(changed, simulation.snapshot()), "Named Save must save directly without replacing the current world"):
		return
	_application.call("_on_world_action", 13)
	_application.call("_on_genotype_file_selected", other)
	if not _check(str(_application.get("_world_path")) == other and FileAccess.file_exists(other), "Successful Save As must change the authoritative filename"):
		return

	for operation: String in ["create", "reset"]:
		_application.call("_request_world_decision", operation, PackedInt64Array([17, 900, 700, 4]))
		_application.call("_world_decision_action", &"save")
		if not _check(str(_application.get("_decision_operation")).is_empty() and not bool(_application.get("_awaiting_world_save")) and _same_model(changed, simulation.snapshot()), "Save before " + operation + " must only save the existing world"):
			return

	_application.call("_on_world_action", 11)
	_application.call("_on_genotype_file_selected", path)
	await process_frame
	if not _check(str(_application.get("_world_path")) == path and bool(simulation.snapshot()["paused"]) and _same_model(changed, simulation.snapshot()) and _laboratories() == laboratories and not is_instance_valid(_application.get("_statistics_window")), "Successful Open must publish the selected world, close statistics, and retain laboratories"):
		return
	_application.call("_open_statistics")
	var cancel_statistics: Window = _application.get("_statistics_window") as Window
	_application.call("_toggle_pause")
	_application.call("_on_world_action", 11)
	_application.call("_finish_file_operation")
	if not _check(bool(simulation.snapshot()["paused"]) and str(_application.get("_world_path")) == path and _application.get("_statistics_window") == cancel_statistics and bool(simulation.snapshot()["statistics_open"]) and _laboratories() == laboratories, "Open cancellation must preserve statistics, laboratories, filename, and source-compatible paused state"):
		return

	_application.call("_request_world_decision", "reset")
	_application.call("_confirm_world_decision")
	if not _check(str(_application.get("_world_path")) == path and int(simulation.snapshot()["tick"]) == 0, "Reset must retain the current filename"):
		return
	if not (await _creation_feedback(simulation, path)):
		return
	_application.connect("simulation_closed", _record_close)
	_application.connect("close_cancelled", _record_cancel)
	_application.call("_toggle_pause")
	if not _check(DirAccess.make_dir_recursive_absolute(preferences_path) == OK, "Cannot create the owned preference destination failure"):
		return
	_application.call("_request_world_decision", "quit")
	_application.call("_world_decision_action", &"save")
	if not _check(_pending_preference_quit(simulation, path, 0), "Named Quit Save must remain paused during preference recovery"):
		return
	_application.call("_retry_preference_close")
	if not _check(_pending_preference_quit(simulation, path, 0), "Failed preference retry must preserve the pending Quit Save"):
		return
	_application.call("_cancel_preference_close")
	if not _check(not bool(simulation.snapshot()["paused"]) and str(_application.get("_decision_operation")).is_empty() and str(_application.get("_preference_close_kind")).is_empty() and not bool(_application.get("_awaiting_world_save")) and _cancel_events == 1 and _close_events == 0, "Cancelling preference recovery must restore the original running world"):
		return
	_application.call("_request_world_decision", "quit")
	_application.call("_world_decision_action", &"save")
	if not _check(_pending_preference_quit(simulation, path, 1), "Repeated named Quit Save must enter preference recovery again"):
		return
	if not _check(DirAccess.remove_absolute(preferences_path) == OK, "Cannot remove the owned preference failure fixture"):
		return
	_application.call("_retry_preference_close")
	if not _check(_close_events == 1 and _cancel_events == 1 and str(_application.get("_decision_operation")).is_empty() and str(_application.get("_preference_close_kind")).is_empty() and not bool(_application.get("_awaiting_world_save")) and not (_application.get("_preference_failure") as ConfirmationDialog).visible and bool(_application.get("_shutting_down")) and _application.get("_simulation") == null and not bool(simulation.preferences_snapshot()["world_ready"]) and FileAccess.file_exists(preferences_path) and str(_application.get("_world_path")) == path, "Successful preference retry must complete Quit Save once and release the Main World owner"):
		return
	_application.call("shutdown")
	_application.queue_free()
	print("BGR_WORLD_FILE_BOUNDARY_PASS " + str(_checks))
	quit(0)


func _creation_feedback(simulation: BgrSimulation, path: String) -> bool:
	var held: Dictionary = simulation.snapshot()
	var values: PackedInt64Array = PackedInt64Array([827364, 1000, 750, 8])
	_application.call("_request_world_decision", "create", values)
	_application.call("_cancel_world_decision")
	if not _check(_same_model(held, simulation.snapshot()) and str(_application.get("_world_path")) == path, "Cancel before confirmation retains the world and filename"):
		return false
	_application.call("_request_world_decision", "create", values)
	_application.call("_confirm_world_decision")
	var dialog: ConfirmationDialog = _application.get("_decision_dialog") as ConfirmationDialog
	if not _check(bool(_application.get("_creating_world")) and dialog.visible and not dialog.dialog_hide_on_ok and dialog.dialog_text == str(preload("res://ui/main_labels.gd").message("R_INITIALIZING", str(_application.get("_main_language")))) and dialog.get_ok_button().disabled and dialog.get_cancel_button().disabled and (_application.get("_save_world_button") as Button).disabled and _same_model(held, simulation.snapshot()), "Publish localized busy feedback before native creation"):
		return false
	_application.call("_confirm_world_decision")
	_application.call("_world_decision_action", &"save")
	if not _check(str(_application.get("_file_operation")).is_empty() and not bool(_application.get("_awaiting_world_save")) and _same_model(held, simulation.snapshot()), "Busy confirmation and Save cannot start another operation"):
		return false
	_application.call("_cancel_world_decision")
	await RenderingServer.frame_post_draw
	await process_frame
	if not _check(not bool(_application.get("_creating_world")) and not dialog.visible and _same_model(held, simulation.snapshot()), "Cancellation during presentation prevents later creation"):
		return false
	var reference: BgrSimulation = BgrSimulation.new()
	var first: Dictionary = reference.configure(827364, 1200, 800, 32)
	if not _check(bool(first["ok"]) and _same_model(first, held), "Bind the independent reference to the reset source world"):
		return false
	var expected: Dictionary = reference.new_game(1000, 750, 8)
	_application.call("_request_world_decision", "create", values)
	_application.call("_confirm_world_decision")
	_application.call("_cancel_world_decision")
	_application.call("_request_world_decision", "create", values)
	values[1] = 1100
	_application.call("_confirm_world_decision")
	_application.call("_confirm_world_decision")
	await RenderingServer.frame_post_draw
	await process_frame
	if not _check(bool(expected["ok"]) and _same_model(expected, simulation.snapshot()) and bool(simulation.snapshot()["paused"]) and str(_application.get("_world_path")) == path and not bool(_application.get("_creating_world")) and not dialog.visible, "One captured creation preserves the RNG stream, paused state, and filename after cancellation and retry"):
		return false
	held = simulation.snapshot()
	_application.call("_request_world_decision", "create", PackedInt64Array([827364, 0, 750, 8]))
	_application.call("_confirm_world_decision")
	await RenderingServer.frame_post_draw
	await process_frame
	if not _check(_same_model(held, simulation.snapshot()) and (_application.get("_error_label") as Label).visible and not bool(_application.get("_creating_world")) and not dialog.visible and not dialog.get_ok_button().disabled and not dialog.get_cancel_button().disabled and not (_application.get("_save_world_button") as Button).disabled, "Rejected creation restores controls and preserves the current error contract"):
		return false
	var closing_simulation: BgrSimulation = BgrSimulation.new()
	if not _check(bool(closing_simulation.preferences_initialize()["ok"]), "Initialize the separate shutdown-yield owner"):
		return false
	var closing: Control = Main.new()
	closing.set("startup_seed_text", "7")
	closing.set("startup_dimensions", Vector2i(96, 96))
	closing.set("startup_attempts", 0)
	if not _check(bool(closing.call("initialize_session", closing_simulation)), "Supply the separate actual simulation owner"):
		return false
	root.add_child(closing)
	closing.set_process(false)
	closing.call("_request_world_decision", "create", PackedInt64Array([7, 96, 96, 4]))
	closing.call("_confirm_world_decision")
	closing.call("shutdown", true)
	await RenderingServer.frame_post_draw
	await process_frame
	var stopped: bool = bool(closing.get("_shutting_down")) and closing.get("_simulation") == null and not bool(closing.get("_creating_world")) and not bool(closing_simulation.preferences_snapshot()["world_ready"])
	closing.queue_free()
	var removal_deadline: int = Time.get_ticks_msec() + 3000
	while is_instance_valid(closing) and Time.get_ticks_msec() < removal_deadline:
		await process_frame
	return _check(stopped and not is_instance_valid(closing), "Shutdown during presentation prevents resurrection and removes the test owner")


func _record_close() -> void:
	_close_events += 1


func _record_cancel() -> void:
	_cancel_events += 1


func _pending_preference_quit(simulation: BgrSimulation, path: String, expected_cancels: int) -> bool:
	return str(_application.get("_file_operation")).is_empty() and str(_application.get("_decision_operation")) == "quit" and str(_application.get("_preference_close_kind")) == "quit" and bool(_application.get("_awaiting_world_save")) and (_application.get("_preference_failure") as ConfirmationDialog).visible and bool(simulation.snapshot()["paused"]) and str(_application.get("_world_path")) == path and _close_events == 0 and _cancel_events == expected_cancels


func _laboratories() -> int:
	var count: int = 0
	for child: Node in _application.get_children():
		if child.get_script() == preload("res://ui/laboratory_window.gd"):
			count += 1
	return count


func _same_model(left: Dictionary, right: Dictionary) -> bool:
	for key: String in ["seed", "width", "height", "initial_attempts", "tick", "population", "oxygen", "carbon_dioxide", "methane", "segments", "colors", "alphas", "organism_ids"]:
		if left.get(key) != right.get(key):
			return false
	return true


func _check(condition: bool, message: String) -> bool:
	_checks += 1
	if not condition:
		printerr("BGR_WORLD_FILE_BOUNDARY_FAIL " + message)
		if is_instance_valid(_application):
			_application.call("shutdown")
		quit(1)
	return condition
