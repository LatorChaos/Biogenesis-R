extends SceneTree

const Main = preload("res://ui/main.gd")
const Pacing = preload("res://ui/adaptive_pacing.gd")
const Preferences = preload("res://ui/display_preferences.gd")
const Labels = preload("res://ui/main_labels.gd")

class ClockMain extends Main:
	var now_usec: int = 4000000

	func _clock_usec() -> int:
		return now_usec

var _checks: int = 0
var _failures: Array[String] = []


func _initialize() -> void:
	_run.call_deferred()


func _check(condition: bool, description: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(description)


func _slow_observation(application: ClockMain) -> void:
	var pacing: Pacing = application.get("_pacing") as Pacing
	pacing.current_delay_ms = 711
	pacing.failed_time = 1
	application.set("_pacing_sample_usec", application.now_usec - 2000000)
	application.set("_last_clock_usec", application.now_usec)
	application.set("_pending_ticks", 7.75)
	application.call("_observe_pacing", 1, application.now_usec)


func _check_continuous_pacing() -> void:
	var pacing: Pacing = Pacing.new()
	pacing.reset(74)
	for index: int in range(10):
		pacing.observe_continuous(97000, 1, 74, true)
	_check(pacing.current_delay_ms == 74, "A partial busy window does not change pace")
	pacing.clear_continuous_sample()
	pacing.observe_continuous(500000, 1, 70, true)
	for index: int in range(5):
		pacing.observe_continuous(50000, 1, 70, true)
		pacing.observe_continuous(172000, 2, 70, true)
	_check(pacing.current_delay_ms == 75, "Weight by completed ticks and exclude the interval entering backlog")
	pacing.observe_continuous(500000, 1, 70, true)
	_check(pacing.current_delay_ms == 75, "An isolated slow completion does not change pace immediately")

	pacing.reset(5)
	for index: int in range(30):
		pacing.observe_continuous(8000, 1, 2, false)
	_check(pacing.current_delay_ms == 2, "Caught-up batches recover to the requested pace despite idle intervals")
	for index: int in range(10):
		pacing.observe_continuous(8000, 1, 2, false)
	_check(pacing.current_delay_ms == 2, "A lightly loaded world stays at the requested pace")

	pacing.reset(1)
	for index: int in range(11):
		pacing.observe_continuous(1500, 1, 1, true)
	_check(pacing.current_delay_ms == 3, "Round up service time and reserve headroom at the 1 ms boundary")
	for index: int in range(10):
		pacing.observe_continuous(3000000000000, 1, 1, true)
	_check(pacing.current_delay_ms == Pacing.MAX_DELAY_MS, "Sustained extreme timing stays within the delay limit")


func _run() -> void:
	_check_continuous_pacing()
	var simulation: BgrSimulation = BgrSimulation.new()
	_check(bool(simulation.preferences_initialize()["ok"]), "Initialize isolated native preferences")
	var application: ClockMain = ClockMain.new()
	application.startup_seed_text = "827364"
	application.startup_dimensions = Vector2i(96, 96)
	application.startup_attempts = 32
	_check(application.initialize_session(simulation), "Use an actual Rust session")
	root.add_child(application)
	application.set_process(false)
	await process_frame
	var control: BgrSimulation = BgrSimulation.new()
	_check(bool(control.start_session(827364, 96, 96, 32)["ok"]), "Create the independent unchanged control")
	control.configure_automatic_backups(OS.get_user_data_dir(), "")
	_check(simulation.snapshot() == control.snapshot(), "Initial actual and control worlds agree")
	var checkbox: CheckBox = application.find_child("ContinuousDisplay", true, false) as CheckBox
	_check(checkbox != null and checkbox.button_pressed and bool(application.get("_continuous_display")), "Default policy and Tools checkbox are continuous")
	_check(checkbox.custom_minimum_size.y >= 48.0 and not checkbox.tooltip_text.is_empty(), "Tools exposes a touch-sized checkbox and explanation")
	for language: String in ["en", "es", "ca", "de", "pt", "ru"]:
		_check(not Labels.message("R_CONTINUOUS_DISPLAY", language).begins_with("!")
			and not Labels.message("R_CONTINUOUS_DISPLAY_HELP", language).begins_with("!"), "Policy labels exist for " + language)
	(application.get("_pause_button") as Button).pressed.emit()
	control.apply_commands(PackedInt32Array([1]))
	var before: Dictionary = simulation.snapshot()
	var pacing: Pacing = application.get("_pacing") as Pacing
	pacing.reset(74)
	application.set("_pacing_sample_usec", application.now_usec)
	application.set("_pending_ticks", 7.75)
	for index: int in range(11):
		application.now_usec += 97000
		application.set("_last_clock_usec", application.now_usec - (7400 if index == 10 else 0))
		application.call("_observe_pacing", 1, application.now_usec)
	_check(pacing.current_delay_ms == 98, "Continuous pace leaves headroom above measured 97 ms service")
	_check(not bool(simulation.snapshot()["paused"]) and not bool(application.get("_overload_paused")), "Continuous observations leave the world running")
	_check(simulation.snapshot() == before and absf(float(application.get("_pending_ticks")) - 7.85) < 0.000000001, "Pace changes preserve the world and accrue fractional debt at the previous period")
	application.call("_accept_result", simulation.advance_budgeted(1, 8000))
	control.advance_budgeted(1, 8000)
	var actual: Dictionary = simulation.snapshot()
	var expected: Dictionary = control.snapshot()
	var differences: Array[String] = []
	for key: String in actual:
		if actual[key] != expected.get(key):
			differences.append(key)
	print("BGR_POLICY_MODEL_DIFFERENCES " + JSON.stringify(differences))
	_check(actual == expected, "Subsequent biological ticks match the unchanged control")
	checkbox.button_pressed = false
	var stored: Preferences = Preferences.new()
	_check(bool(stored.load_settings()["ok"]) and stored.values["continuous_display"] == false, "The real checkbox persists strict false")
	_slow_observation(application)
	control.apply_commands(PackedInt32Array([0]))
	_check(bool(simulation.snapshot()["paused"]) and bool(application.get("_overload_paused")), "Disabled continuous mode retains legacy automatic overload Pause")
	_check(simulation.snapshot() == control.snapshot() and float(application.get("_pending_ticks")) == 7.75, "Legacy Pause preserves the model and queued debt")
	var sample: Dictionary = application.call("_endurance_sample")
	_check(sample["continuous_display"] == false and sample["overload_paused"] == true, "Cached diagnostics identify the legacy overload Pause")
	before = simulation.snapshot()
	checkbox.button_pressed = true
	_check(simulation.snapshot() == before and float(application.get("_pending_ticks")) == 7.75, "Enabling continuous mode never resumes a paused world or clears debt")
	_check(bool(stored.load_settings()["ok"]) and stored.values["continuous_display"] == true, "The real checkbox persists strict true")
	(application.get("_pause_button") as Button).pressed.emit()
	control.apply_commands(PackedInt32Array([1]))
	_check(not bool(application.get("_overload_paused")) and pacing.current_delay_ms == 2
		and simulation.snapshot() == control.snapshot(), "Only explicit Resume resets overload pacing through the existing path")
	(application.get("_pause_button") as Button).pressed.emit()
	control.apply_commands(PackedInt32Array([0]))
	checkbox.button_pressed = false
	checkbox.button_pressed = true
	_slow_observation(application)
	_check(bool(simulation.snapshot()["paused"]) and not bool(application.get("_overload_paused"))
		and simulation.snapshot() == control.snapshot(), "A manual Pause survives policy changes and slow observations")
	(application.get("_step_button") as Button).pressed.emit()
	control.apply_commands(PackedInt32Array([2]))
	_check(simulation.snapshot() == control.snapshot() and float(application.get("_pending_ticks")) == 7.75, "Manual Step retains existing biology and paused debt")
	application.set("_runtime_failed", true)
	before = simulation.snapshot()
	checkbox.button_pressed = false
	checkbox.button_pressed = true
	_check(bool(application.get("_runtime_failed")) and simulation.snapshot() == before, "Policy changes do not clear runtime failure or resume")
	sample = application.call("_endurance_sample")
	_check(sample["continuous_display"] == true and sample["overload_paused"] == false
		and sample["paused"] == true, "Cached diagnostics distinguish manual Pause from policy")
	application.queue_free()
	await process_frame
	print("BGR_CONTINUOUS_DISPLAY " + JSON.stringify({"result": "PASS" if _failures.is_empty() else "FAIL", "checks": _checks, "failures": _failures}))
	quit(0 if _failures.is_empty() else 1)
