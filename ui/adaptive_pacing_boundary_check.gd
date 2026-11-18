extends SceneTree

const Main = preload("res://ui/main.gd")
const Pacing = preload("res://ui/adaptive_pacing.gd")
const Labels = preload("res://ui/main_labels.gd")

class ClockMain extends Main:
	var now_usec: int = 1000000
	var completed_work_usec: int = 0

	func _clock_usec() -> int:
		return now_usec

	func _accept_result(result: Dictionary) -> bool:
		if completed_work_usec > 0 and int(result.get("tick", 0)) > int(_state.get("tick", 0)):
			now_usec += completed_work_usec
			completed_work_usec = 0
		return super._accept_result(result)

var _checks: int = 0
var _failures: Array[String] = []
var _main: ClockMain
var _simulation: BgrSimulation


func _initialize() -> void:
	_run.call_deferred()


func _check(condition: bool, description: String) -> bool:
	_checks += 1
	if not condition:
		_failures.append(description)
	return condition


func _run() -> void:
	var golden: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://ui/adaptive_pacing_expected.json"))
	assert(golden["schema"] == 1 and golden["rows"].size() == 2662)
	assert(golden["source_fragment_sha256"] == "bdc1c58cbc95c7b8e87b8dab21aa72cd6ba505c2924b8878b20bcbda07791131")
	var pacing: Pacing = Pacing.new()
	for row: Dictionary in golden["rows"]:
		var input: Dictionary = row["input"]
		var expected: Dictionary = row["output"]
		pacing.current_delay_ms = int(input["current_delay_ms"])
		pacing.failed_time = int(input["failed_time"])
		pacing.observe(float(input["elapsed_ms"]), int(input["requested_delay_ms"]))
		_check(pacing.current_delay_ms == int(expected["current_delay_ms"]) and pacing.failed_time == int(expected["failed_time"]) and pacing.restarted == (int(expected["restarts"]) == 1) and pacing.pause_requested == (int(expected["pause"]) == 1) and (pacing.current_delay_ms if pacing.restarted else -1) == int(expected["scheduled_delay_ms"]), "Source pacing vector " + str(row["id"]))
	_simulation = BgrSimulation.new()
	_check(_simulation.preferences_initialize()["ok"], "Initialize the owned preference profile")
	_main = ClockMain.new()
	_main.startup_seed_text = "827364"
	_main.startup_dimensions = Vector2i(96, 96)
	_main.startup_attempts = 32
	_check(_main.initialize_session(_simulation), "Initialize actual Main")
	root.add_child(_main)
	_main.set_process(false)
	(_main.get("_continuous_display_checkbox") as CheckBox).button_pressed = false
	var control: BgrSimulation = BgrSimulation.new()
	_check(control.start_session(827364, 96, 96, 32)["ok"], "Initialize an independent control simulation")
	control.configure_automatic_backups(OS.get_user_data_dir(), "")
	_check(_simulation.snapshot() == control.snapshot(), "Initial models agree before pacing changes")
	(_main.get("_pause_button") as Button).pressed.emit()
	control.apply_commands(PackedInt32Array([1]))
	var live: Pacing = _main.get("_pacing") as Pacing
	live.current_delay_ms = 711
	live.failed_time = 1
	_main.set("_pending_ticks", 1.25)
	_main.now_usec = 1000000
	_main.completed_work_usec = 2000000
	_main.set("_last_clock_usec", _main.now_usec)
	_main.set("_pacing_sample_usec", 1000000)
	_main.call("_process", 0.0)
	control.advance_budgeted(1, 8000)
	control.apply_commands(PackedInt32Array([0]))
	var expected_debt: float = 0.25 + 2000.0 / 711.0
	_check(absf(float(_main.get("_pending_ticks")) - expected_debt) < 0.000000001, "Batch work time enters debt before pacing changes or pauses")
	_check(bool(_main.get("_overload_paused")) and _simulation.snapshot()["paused"] and live.current_delay_ms == 1066, "Sustained overload pauses the actual owner")
	_check(_simulation.snapshot() == control.snapshot(), "Overload preserves the normal single-tick command result")
	_check(absf(float(_main.get("_pending_ticks")) - expected_debt) < 0.000000001, "Completed ticks are paid while fractional debt remains")
	(_main.get("_step_button") as Button).pressed.emit()
	control.apply_commands(PackedInt32Array([2]))
	_check(_simulation.snapshot() == control.snapshot() and absf(float(_main.get("_pending_ticks")) - expected_debt) < 0.000000001, "Manual Step preserves paused debt and normal simulation behavior")
	(_main.get("_pause_button") as Button).pressed.emit()
	control.apply_commands(PackedInt32Array([1]))
	_check(not bool(_main.get("_overload_paused")) and live.current_delay_ms == 2 and live.failed_time == 0 and absf(float(_main.get("_pending_ticks")) - expected_debt) < 0.000000001, "Resume retries adaptation without discarding debt")
	_check(_simulation.snapshot() == control.snapshot(), "Resume preserves the normal model and random stream")
	_main.set("_pending_ticks", 17.375)
	_apply_settings(7)
	_check(live.current_delay_ms == 7 and float(_main.get("_pending_ticks")) == 17.375, "Settings preserve owed tick quantity across period changes")
	_main.now_usec += 14000
	_main.call("_accrue_clock")
	_check(float(_main.get("_pending_ticks")) == 19.375, "Real clock accrual uses the effective tick period")
	_main.set("_pending_ticks", 17.375)
	_apply_settings(1)
	live.failed_time = 1
	_main.set("_pacing_sample_usec", 1000)
	_main.call("_observe_pacing", 1, 11000)
	_check(live.current_delay_ms == 2 and not bool(_main.get("_overload_paused")) and float(_main.get("_pending_ticks")) == 17.375, "Live delay-one correction permits progress without changing debt")
	_apply_settings(2000)
	_main.set("_pacing_sample_usec", 1000)
	_main.call("_observe_pacing", 1, 2001000)
	_check(live.pause_requested and not bool(_main.get("_overload_paused")) and not _simulation.snapshot()["paused"], "A deliberately requested slow period does not falsely pause")
	_apply_settings(2)
	_main.set("_pending_ticks", 7.75)
	_main.call("_reset_pacing")
	_main.set("_pacing_sample_usec", 1000)
	_main.call("_observe_pacing", 8, 17000)
	_check(live.current_delay_ms == 2 and live.failed_time == 0 and float(_main.get("_pending_ticks")) == 7.75, "Batch normalization uses completed biological ticks")
	_main.call("_observe_pacing", 0, 18000)
	_check(live.current_delay_ms == 2 and int(_main.get("_pacing_sample_usec")) == 0, "Empty batches do not invent timing observations")
	var next_main: Dictionary = _simulation.new_game(96, 96, 16)
	var next_control: Dictionary = control.new_game(96, 96, 16)
	_check(next_main["ok"] and next_control["ok"] and next_main["segments"] == next_control["segments"] and next_main["colors"] == next_control["colors"] and next_main["alphas"] == next_control["alphas"], "Pacing does not consume the retained random stream")
	_main.call("_accept_result", next_main)
	_main.set("_pending_ticks", 123.5)
	_check(_main.call("_apply_reset") and float(_main.get("_pending_ticks")) == 0.0 and live.current_delay_ms == 2, "Explicit Reset clears old-world debt and adaptation")
	(_main.get("_pause_button") as Button).pressed.emit()
	_main.set("_pending_ticks", 100.5)
	var before_batch: int = int(_simulation.snapshot()["tick"])
	_main.call("_process", 0.0)
	var completed_batch: int = int(_simulation.snapshot()["tick"]) - before_batch
	_check(completed_batch > 0 and completed_batch <= 16 and float(_main.get("_pending_ticks")) == 100.5 - completed_batch, "Catch-up remains bounded and preserves every unpaid tick")
	for language: String in ["en", "es", "ca", "de", "pt", "ru"]:
		_check(Labels.DATA[language].has("R_OVERLOAD_PAUSED") and Labels.DATA[language].has("R_ADAPTIVE_TIMING"), "Pacing feedback exists in " + language)
	_main.shutdown()
	_main.queue_free()
	await process_frame
	print("ADAPTIVE_PACING_RESULT ", JSON.stringify({"status": "PASS" if _failures.is_empty() else "FAIL", "checks": _checks, "source_vectors": 2662, "controlled_clock": true, "real_Rust_owner": true, "failures": _failures}))
	quit(0 if _failures.is_empty() else 1)


func _apply_settings(delay_ms: int) -> void:
	var draft: BgrSettingsDraft = _simulation.settings_open()
	var state: Dictionary = draft.snapshot()
	var texts: PackedStringArray = state["texts"]
	var keys: PackedStringArray = state["keys"]
	texts[keys.find("DELAY")] = str(delay_ms)
	_check(draft.replace_controls({"texts": texts, "locale": state["locale"], "automatic_backup": state["automatic_backup"], "hardware": state["hardware"], "disable_framebuffer": state["disable_framebuffer"]})["ok"], "Replace the real delay draft")
	var result: Dictionary = _simulation.settings_apply(draft, Callable(_main, "_settings_locale_changed"))
	if not _check(result["ok"], "Apply the actual settings transaction"):
		return
	_main.call("_settings_applied", result)
