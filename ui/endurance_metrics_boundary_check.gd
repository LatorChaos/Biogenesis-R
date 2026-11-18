extends SceneTree

const Metrics = preload("res://ui/endurance_metrics.gd")
const Main = preload("res://ui/main.gd")
const StartupArguments = preload("res://ui/startup_arguments.gd")

class UncertainWriteMetrics extends Metrics:
	var inject_failure: bool = false
	var partial: bool = false
	var write_attempts: int = 0
	var first_unconfirmed_bytes: PackedByteArray = PackedByteArray()

	func _write(data: PackedByteArray) -> bool:
		write_attempts += 1
		if not inject_failure:
			return super._write(data)
		var emitted: PackedByteArray = data.slice(0, mini(17, data.size())) if partial else data
		if first_unconfirmed_bytes.is_empty():
			first_unconfirmed_bytes = emitted.duplicate()
		_file.store_buffer(emitted)
		_file.flush()
		assert(_file.get_error() == OK, "Fault fixture must retain its injected bytes")
		return false

class NativeClockMetrics extends Metrics:
	var native_now: int = 100

	func _native_clock() -> int:
		return native_now

class NativeUncertainMetrics extends NativeClockMetrics:
	var partial_event: bool = true
	var first_unconfirmed: PackedByteArray = PackedByteArray()
	var uncertain_attempts: int = 0

	func _native_write(file: FileAccess, data: PackedByteArray, flush: bool) -> bool:
		if file != _native_events:
			return super._native_write(file, data, flush)
		uncertain_attempts += 1
		first_unconfirmed = data.slice(0, mini(17, data.size())) if partial_event else data.duplicate()
		file.store_buffer(first_unconfirmed)
		file.flush()
		assert(file.get_error() == OK)
		return false

var _checks: int = 0
var _directory: String = ""


func _check(value: bool, message: String) -> void:
	_checks += 1
	if not value:
		printerr("BGR_ENDURANCE_BOUNDARY_FAIL " + message)
		quit(1)
		assert(false, message)


func _rows(name: String) -> Array[Dictionary]:
	var result: Array[Dictionary] = []
	var text: String = FileAccess.get_file_as_string(_directory.path_join(name).path_join("application.jsonl"))
	for line: String in text.split("\n", false):
		var parsed: Variant = JSON.parse_string(line)
		_check(parsed is Dictionary, "Output must contain complete JSON objects")
		result.append(parsed)
	return result


func _initialize() -> void:
	_directory = OS.get_environment("BGR_METRICS_CHECK_OUTPUT")
	_check(_directory.begins_with("/") and DirAccess.dir_exists_absolute(_directory), "Require an existing private output parent")
	_check(not bool(Metrics.options(PackedStringArray())["enabled"]), "Default mode stays disabled")
	for arguments: PackedStringArray in [
		PackedStringArray(["--bgr-endurance-dir="]),
		PackedStringArray(["--bgr-endurance-dir=relative"]),
		PackedStringArray(["--bgr-endurance-dir=/a/../b"]),
		PackedStringArray(["--bgr-endurance-dir=/a", "--bgr-endurance-dir=/b"]),
		PackedStringArray(["--bgr-endurance-interval=1"]),
		PackedStringArray(["--bgr-endurance-dir"])
	]:
		var invalid: Dictionary = Metrics.options(arguments)
		_check(not bool(invalid["enabled"]) and not str(invalid["error"]).is_empty(), "Reject invalid options explicitly")
	var option: String = "--bgr-endurance-dir=" + _directory.path_join("valid")
	_check(bool(Metrics.options(PackedStringArray([option]))["enabled"]), "Accept one absolute directory option")
	var replacement_name: String = "replacement-\ufffd"
	var replacement_directory: String = _directory.path_join(replacement_name)
	var replacement_option: Dictionary = Metrics.options(PackedStringArray(["--bgr-endurance-dir=" + replacement_directory]))
	_check(bool(replacement_option["enabled"]) and replacement_option["directory"] == replacement_directory, "Preserve a valid replacement-character path")
	var gameplay: Dictionary = StartupArguments.parse(Metrics.gameplay_arguments(PackedStringArray(["--launcher", option, "827364"])))
	_check(bool(gameplay["valid"]) and bool(gameplay["launcher_only"]) and str(gameplay["seed_text"]) == "827364", "Preserve launcher and seed parsing")
	_check(not bool(StartupArguments.parse(Metrics.gameplay_arguments(PackedStringArray(["--unrelated-option"])))["valid"]), "Retain unrelated unknown-option rejection")

	var main: Main = Main.new()
	main._start_endurance()
	_check(main.get("_endurance") == null, "Actual Main startup creates no disabled collector")
	var crisp: CheckButton = CheckButton.new()
	crisp.button_pressed = true
	main.set("_crisp_lines", crisp)
	main.set("_state", {"seed": 827364, "width": 7000.0, "height": 7000.0, "initial_attempts": 12500, "tick": 45, "world_time": 5, "population": 12000, "paused": false})
	main.set("_pending_ticks", 7.25)
	main.set("_network_packet", {
		"pending_command_jobs": 2, "closing": true, "job_limit": 4, "incoming_limit": 8,
		"connections": [
			{"queued_incoming": 3, "incoming_candidate": true, "pending_outgoing": 1, "address": "private-peer"},
			{"queued_incoming": 2, "incoming_candidate": false, "pending_outgoing": 4}
		]
	})
	var sample: Dictionary = main._endurance_sample()
	_check(int(sample["seed"]) == 827364 and int(sample["tick"]) == 45 and int(sample["world_time"]) == 5, "Use actual Main cached workload fields")
	_check(int(sample["population"]) == 12000 and bool(sample["crisp_lines"]), "Retain population and quality identity")
	_check(sample["history_counts"] == null and sample["history_counts_status"] == "unavailable_in_cached_state", "Do not invent history counts")
	_check(sample["network"]["queued_incoming"] == 5 and sample["network"]["incoming_candidates"] == 1 and sample["network"]["pending_outgoing"] == 5, "Sum actual bounded peer queue fields")
	_check(sample["network"]["pending_command_jobs"] == 2 and bool(sample["network"]["closing"]) and sample["network"]["job_limit"] == 4, "Separate command jobs, closing worker, and limits")
	_check(not JSON.stringify(sample).contains("private-peer"), "Exclude addresses from output")

	var replacement: Metrics = Metrics.new()
	_check(replacement.start(replacement_directory, 0, sample), "Record into a valid replacement-character directory")
	replacement.finish(1, sample, "shutdown")
	var replacement_rows: Array[Dictionary] = _rows(replacement_name)
	_check(replacement_rows.size() == 2 and replacement_rows[-1]["kind"] == "final" and bool(replacement_rows[-1]["collection_ok"]), "Finalize replacement-character output normally")

	var recorder: Metrics = Metrics.new()
	_check(recorder.start(_directory.path_join("valid"), 100, sample), "Start a fresh stream")
	recorder.observe_dispatch(100, false, false)
	recorder.observe_debt(7.25)
	recorder.observe_batch(3, 400, true)
	recorder.observe_dispatch(150, true, false)
	recorder.observe_debt(10.5)
	recorder.observe_batch(1, 900, false)
	recorder.observe_dispatch(350, false, true)
	_check(not recorder.due(60_000_099) and recorder.due(60_000_100), "Use the fixed minute boundary")
	recorder.record(60_000_100, sample)
	var rows: Array[Dictionary] = _rows("valid")
	_check(rows.size() == 2 and rows[0]["kind"] == "start" and rows[1]["kind"] == "interval", "Emit start and one minute row")
	var interval: Dictionary = rows[1]
	_check(interval["dispatch_count"] == 3 and interval["dispatch_gap_count"] == 2, "Count dispatches independently from measured gaps")
	_check(interval["dispatch_gap_sum_usec"] == 250 and interval["dispatch_gap_max_usec"] == 200 and interval["dispatch_gap_mean_usec"] == 125.0, "Keep exact gap sum, maximum, and mean")
	_check(interval["advance_and_accept_batch_count"] == 2 and interval["advance_and_accept_completed_ticks"] == 4 and interval["advance_and_accept_failed_count"] == 1 and interval["advance_and_accept_batch_max_usec"] == 900, "Aggregate actual batch observations")
	_check(interval["pending_ticks_max"] == 10.5 and bool(interval["paused_seen"]) and bool(interval["runtime_failure_seen"]), "Retain debt peaks and observed states")
	recorder.record(60_000_101, sample)
	_check(recorder.rows_written == 2, "Never persist a high-frequency heartbeat")
	recorder.observe_dispatch(60_000_150, false, false)
	recorder.finish(60_000_200, sample, "shutdown")
	rows = _rows("valid")
	_check(rows.size() == 3 and rows[2]["kind"] == "final" and bool(rows[2]["collection_ok"]), "Flush a bounded terminal row")
	_check(rows[2]["dispatch_count"] == 1 and rows[2]["advance_and_accept_batch_count"] == 0 and rows[2]["pending_ticks_max"] == 0.0, "Reset interval aggregates after each row")
	var finalized: PackedByteArray = FileAccess.get_file_as_bytes(_directory.path_join("valid/application.jsonl"))
	recorder.finish(60_000_300, sample, "shutdown")
	_check(FileAccess.get_file_as_bytes(_directory.path_join("valid/application.jsonl")) == finalized, "Finalization is idempotent")
	var duplicate: Metrics = Metrics.new()
	_check(not duplicate.start(_directory.path_join("valid"), 0, sample), "Refuse existing output directories")
	_check(FileAccess.get_file_as_bytes(_directory.path_join("valid/application.jsonl")) == finalized, "Existing output remains unchanged")
	var existing: String = _directory.path_join("existing-file")
	_check(not FileAccess.file_exists(existing) and not DirAccess.dir_exists_absolute(existing), "Refusal fixture must have a fresh path")
	var sentinel: FileAccess = FileAccess.open(existing, FileAccess.WRITE)
	_check(sentinel != null, "Create the owned refusal fixture")
	sentinel.store_string("preserve")
	sentinel.close()
	var file_refusal: Metrics = Metrics.new()
	_check(not file_refusal.start(existing, 0, sample) and FileAccess.get_file_as_string(existing) == "preserve", "Refuse existing files without overwrite")
	var missing_parent: Metrics = Metrics.new()
	_check(not missing_parent.start(_directory.path_join("absent/child"), 0, sample), "Report directory creation failure")
	var malformed: Dictionary = sample.duplicate(true)
	malformed.erase("tick")
	var invalid_sample: Metrics = Metrics.new()
	_check(not invalid_sample.start(_directory.path_join("invalid-sample"), 0, malformed), "Reject malformed samples before writing")
	_check(not DirAccess.dir_exists_absolute(_directory.path_join("invalid-sample")), "Invalid startup creates no output")

	for name: String in ["row-cap", "byte-cap", "row-size", "write-failure", "invalid-row", "clock-failure", "repeat-start"]:
		var limited: Metrics = Metrics.new()
		_check(limited.start(_directory.path_join(name), 100, sample), "Start an isolated failure stream")
		match name:
			"row-cap": limited.rows_written = Metrics.MAX_ROWS - 1
			"byte-cap": limited.bytes_written = Metrics.MAX_BYTES - Metrics.MAX_ROW_BYTES
			"write-failure": limited._file.close()
			"clock-failure": limited.observe_dispatch(99, false, false)
			"repeat-start": limited.start(_directory.path_join("unused-repeat"), 100, sample)
		var next_sample: Dictionary = sample.duplicate(true)
		if name == "row-size":
			next_sample["oversized_fixture"] = "x".repeat(Metrics.MAX_ROW_BYTES)
		elif name == "invalid-row":
			next_sample["pending_ticks"] = -1.0
		limited.record(60_000_100, next_sample)
		_check(not limited.active and not limited.failure.is_empty(), "Collection failures are explicit and terminal")
		var failure_rows: Array[Dictionary] = _rows(name)
		_check(failure_rows[0]["kind"] == "start", "Retain previously flushed evidence")
		for row: Dictionary in failure_rows:
			_check(row["kind"] != "final", "Never emit success after a failure")
		if name != "write-failure":
			_check(failure_rows[-1]["kind"] == "failure" and not bool(failure_rows[-1]["collection_ok"]), "Use reserved failure capacity")

	var partial: UncertainWriteMetrics = UncertainWriteMetrics.new()
	partial.inject_failure = true
	partial.partial = true
	var partial_path: String = _directory.path_join("partial-first/application.jsonl")
	_check(not partial.start(_directory.path_join("partial-first"), 100, sample), "Fail collection after an injected partial first write")
	var partial_bytes: PackedByteArray = FileAccess.get_file_as_bytes(partial_path)
	_check(partial_bytes == partial.first_unconfirmed_bytes and partial.write_attempts == 1, "Error reporting must not append to a partial row")
	_check(not partial_bytes.is_empty() and not partial_bytes.get_string_from_utf8().contains("\n"), "Preserve the incomplete first row exactly")
	_check(not partial.active and not partial.failure.is_empty() and partial.rows_written == 0, "Partial writes remain unconfirmed terminal failures")
	partial.finish(200, sample, "shutdown")
	partial.record(60_000_100, sample)
	_check(FileAccess.get_file_as_bytes(partial_path) == partial_bytes and partial.write_attempts == 1, "Repeated completion must leave partial evidence unchanged")

	var unconfirmed: UncertainWriteMetrics = UncertainWriteMetrics.new()
	var unconfirmed_path: String = _directory.path_join("unconfirmed-final/application.jsonl")
	_check(unconfirmed.start(_directory.path_join("unconfirmed-final"), 100, sample), "Start the unconfirmed flush fixture")
	var confirmed_bytes: PackedByteArray = FileAccess.get_file_as_bytes(unconfirmed_path)
	unconfirmed.rows_written = Metrics.MAX_ROWS - 1
	unconfirmed.inject_failure = true
	unconfirmed.finish(200, sample, "shutdown")
	var retained_bytes: PackedByteArray = FileAccess.get_file_as_bytes(unconfirmed_path)
	_check(retained_bytes == confirmed_bytes + unconfirmed.first_unconfirmed_bytes, "Error reporting must preserve a fully written unconfirmed row")
	_check(unconfirmed.write_attempts == 2 and retained_bytes.get_string_from_utf8().split("\n", false).size() == 2, "Never append an extra physical row after an uncertain flush")
	_check(not unconfirmed.active and not unconfirmed.failure.is_empty() and unconfirmed.rows_written == Metrics.MAX_ROWS - 1, "An unconfirmed final row must retain terminal failure state")
	unconfirmed.finish(300, sample, "shutdown")
	unconfirmed.record(60_000_100, sample)
	_check(FileAccess.get_file_as_bytes(unconfirmed_path) == retained_bytes and unconfirmed.write_attempts == 2, "Repeated completion must leave unconfirmed evidence unchanged")

	var finite: Metrics = Metrics.new()
	_native_boundary(sample)
	_check(finite.start(_directory.path_join("finite"), 0, sample), "Start finite-duration verification")
	for minute: int in range(1, 1451):
		finite.record(minute * Metrics.INTERVAL_USEC, sample)
	_check(not finite.active and finite.failure.is_empty(), "Stop recording after 24 hours plus cleanup allowance")
	var finite_rows: Array[Dictionary] = _rows("finite")
	_check(finite_rows.size() <= Metrics.MAX_ROWS and finite_rows[-1]["kind"] == "final" and finite_rows[-1]["reason"] == "time_limit", "Duration stays within the fixed row budget")
	_check(finite.bytes_written <= Metrics.MAX_BYTES, "Finite output stays within the fixed byte budget")
	var gap: Metrics = Metrics.new()
	_check(gap.start(_directory.path_join("gap"), 0, sample), "Start a collection-gap fixture")
	gap.record(180_000_000, sample)
	gap.finish(180_000_001, sample, "shutdown")
	var gap_rows: Array[Dictionary] = _rows("gap")
	_check(gap_rows.size() == 3 and gap_rows[1]["missed_minute_boundaries"] == 2, "Expose collection gaps without synthetic catch-up rows")
	crisp.free()
	main.free()
	print("BGR_ENDURANCE_BOUNDARY_OK checks=" + str(_checks))
	quit(0)


func _native_configuration() -> Dictionary:
	return {"seed": 827364, "width": 7000, "height": 7000, "initial_attempts": 512,
		"window_id": 1, "tick": 0, "max_fps": 0, "thread_setting": 1, "vsync_mode": 1,
		"thread_setting_present": true, "thread_setting_status": "PROJECT_SETTING_PRESENT",
		"paused": true, "state_available": true, "fit_enabled": true, "crisp_lines": false,
		"window_size": [1200, 900], "window_position": [0, 0], "canvas_rect": [10, 100, 1100, 600],
		"offset": [0, 0], "zoom": 0.076, "controls": {},
		"screen_refresh_hz": null, "screen_refresh_status": "UNKNOWN"}


func _native_recorder(name: String, sample: Dictionary) -> NativeClockMetrics:
	var recorder: NativeClockMetrics = NativeClockMetrics.new()
	var path: String = _directory.path_join(name)
	_check(recorder.start(path, 100, sample) and recorder.start_native(path, 100), "Start optional native fixture")
	recorder.native_configuration(_native_configuration(), 100, true)
	return recorder


func _native_sample(recorder: NativeClockMetrics, frame: int, now: int, tick: int, paused: bool) -> void:
	recorder.native_now = now + 1
	recorder.native_phase(frame, now, tick, paused, false)


func _native_trace(recorder: NativeClockMetrics, freeze: bool = true) -> void:
	var resume: int = recorder.native_request("space", 110, 0, true)
	recorder.native_command(resume, 120, true, 0, false)
	for index: int in range(6):
		_native_sample(recorder, 10 + index, 130 + index * 100, index * 16, false)
	var pause: int = recorder.native_request("space", 650, 80, false)
	recorder.native_command(pause, 660, true, 80, true)
	_native_sample(recorder, 16, 680, 80, true)
	if freeze:
		_native_sample(recorder, 17, 300680, 80, true)
	recorder.native_now = 300700


func _native_events(name: String) -> Array[Dictionary]:
	var rows: Array[Dictionary] = []
	for line: String in FileAccess.get_file_as_string(_directory.path_join(name).path_join("native-events.jsonl")).split("\n", false):
		var row: Variant = JSON.parse_string(line)
		_check(row is Dictionary, "Keep complete native event rows")
		rows.append(row)
	return rows


func _native_boundary(sample: Dictionary) -> void:
	var cases: int = 0
	var option: String = "--bgr-endurance-dir=" + _directory.path_join("native")
	_check(bool(Metrics.options(PackedStringArray([option, Metrics.NATIVE_OPTION]))["native"]), "Native flag requires explicit opt-in")
	for args: PackedStringArray in [PackedStringArray([Metrics.NATIVE_OPTION]), PackedStringArray([option, Metrics.NATIVE_OPTION, Metrics.NATIVE_OPTION])]:
		_check(not bool(Metrics.options(args)["enabled"]), "Reject missing directory or duplicate native flag")
	var disabled: Metrics = Metrics.new()
	_check(disabled.start(_directory.path_join("native-disabled"), 100, sample), "Start unchanged minute mode")
	_check(not disabled.native_enabled and disabled._native_rows.is_empty(), "No frame payload without native option")
	_check(not FileAccess.file_exists(_directory.path_join("native-disabled/native-frames.tsv")), "No native file without native option")
	disabled.finish(101, sample, "shutdown")
	cases += 1

	var trace: NativeClockMetrics = _native_recorder("native-trace", sample)
	_native_trace(trace)
	_check(not trace.native_active and not trace.native_complete, "Serialize after measured phase and geometry check")
	trace.native_flush_pending()
	_check(trace.native_complete and trace.native_failure.is_empty(), "Confirm complete causal freeze")
	var frames: PackedStringArray = FileAccess.get_file_as_string(_directory.path_join("native-trace/native-frames.tsv")).split("\n", false)
	_check(frames.size() == 8, "Keep every ordered sample including zero tick context")
	var ticks: int = 0
	for index: int in range(frames.size()):
		var fields: PackedStringArray = frames[index].split("\t")
		_check(fields.size() == 9 and int(fields[0]) == index and int(fields[8]) == 1, "Exact frame schema and sequence")
		if index > 0:
			ticks += int(fields[3]) - int(frames[index - 1].split("\t")[3])
	_check(ticks == 80, "Consecutive raw tick increments reconcile independently")
	var events: Array[Dictionary] = _native_events("native-trace")
	_check(events.size() == 9 and events[6]["kind"] == "pause_observed" and events[6]["request"] == 2, "Link exact accepted request to observed frame")
	_check(events[7]["frozen_usec"] == 300000 and events[-1]["collection_ok"], "Keep full freeze and confirmed terminal")
	cases += 1

	var original: PackedByteArray = FileAccess.get_file_as_bytes(_directory.path_join("native-trace/native-events.jsonl"))
	trace.finish_native(300800)
	_check(FileAccess.get_file_as_bytes(_directory.path_join("native-trace/native-events.jsonl")) == original, "Completion is idempotent")
	_check(not trace.start_native(_directory.path_join("different"), 300900), "Refuse repeated or unrelated native output")
	cases += 1

	var rejected: NativeClockMetrics = _native_recorder("native-rejected", sample)
	var request: int = rejected.native_request("space", 110, 0, true)
	rejected.native_command(request, 120, false, 0, true)
	rejected.finish_native(130)
	_check(not rejected.native_complete and not rejected.native_failure.is_empty(), "Failed commands cannot acknowledge pause")
	var incomplete: NativeClockMetrics = _native_recorder("native-incomplete", sample)
	incomplete.finish_native(110)
	_check(not incomplete.native_complete and not incomplete.native_failure.is_empty(), "Shutdown cannot invent a completed collection")
	cases += 1
	for reason: String in ["clock", "frame", "tick"]:
		var invalid: NativeClockMetrics = _native_recorder("native-" + reason, sample)
		_native_sample(invalid, 1, 120, 0, true)
		_native_sample(invalid, 1 if reason == "frame" else 2, 119 if reason == "clock" else 130, -1 if reason == "tick" else 0, true)
		invalid.finish_native(140)
		_check(invalid.native_count == 1 and not invalid.native_complete and not invalid.native_failure.is_empty(), "Keep valid prefix on order failure")
		cases += 1

	var early: NativeClockMetrics = _native_recorder("native-early", sample)
	request = early.native_request("space", 110, 0, true)
	early.native_command(request, 120, true, 0, false)
	_native_sample(early, 1, 130, 0, false)
	request = early.native_request("p", 140, 0, false)
	early.native_command(request, 150, true, 0, true)
	_native_sample(early, 2, 160, 0, true)
	early.finish_native(170)
	_check(not early.native_complete and not early.native_failure.is_empty(), "Pause before80 ticks cannot qualify")
	cases += 1
	var drift: NativeClockMetrics = _native_recorder("native-drift", sample)
	_native_trace(drift)
	drift.native_configuration(_native_configuration(), 300690, true)
	drift.finish_native(300700)
	_check(not drift.native_complete and not drift.native_failure.is_empty(), "A pending dump cannot hide final geometry change")
	drift = _native_recorder("native-frozen-tick-drift", sample)
	_native_trace(drift, false)
	_native_sample(drift, 17, 700, 81, true)
	drift.finish_native(710)
	_check(not drift.native_complete and not drift.native_failure.is_empty(), "Paused tick must stay unchanged through the freeze")
	cases += 1

	var quota: NativeClockMetrics = _native_recorder("native-row-quota", sample)
	for index: int in range(Metrics.NATIVE_ROWS + 1):
		_native_sample(quota, index, 120 + index, 0, true)
	quota.finish_native(5000)
	_check(quota.native_count == 4096 and not quota.native_complete and not quota.native_failure.is_empty(), "Preserve full raw prefix on bounded frame overflow")
	cases += 1
	quota = _native_recorder("native-event-quota", sample)
	for index: int in range(Metrics.NATIVE_EVENTS):
		quota.native_configuration(_native_configuration(), 110 + index, true)
	quota.finish_native(300)
	_check(quota._native_event_count == 64 and not quota.native_complete and not quota.native_failure.is_empty(), "Reserve failure event at control quota")
	quota = _native_recorder("native-byte-quota", sample)
	quota._native_bytes = Metrics.NATIVE_BYTES - 8192
	quota.native_configuration(_native_configuration(), 110, true)
	quota.finish_native(120)
	_check(not quota.native_complete and not quota.native_failure.is_empty(), "Reject total byte quota before appending an unreserved row")
	cases += 1

	for partial: bool in [true, false]:
		var uncertain: NativeUncertainMetrics = NativeUncertainMetrics.new()
		var name: String = "native-partial" if partial else "native-unconfirmed"
		var path: String = _directory.path_join(name)
		uncertain.partial_event = partial
		_check(uncertain.start(path, 100, sample) and uncertain.start_native(path, 100), "Start native uncertain-write fixture")
		uncertain.native_configuration(_native_configuration(), 100, true)
		uncertain.finish_native(110)
		var raw: PackedByteArray = FileAccess.get_file_as_bytes(path.path_join("native-events.jsonl"))
		_check(raw == uncertain.first_unconfirmed and uncertain.uncertain_attempts == 1, "Never append after partial or unconfirmed native write")
		_check(not uncertain.native_complete and not uncertain.native_failure.is_empty(), "Uncertain native flush cannot pass")
	cases += 1

	var malformed: NativeClockMetrics = _native_recorder("native-invalid-config", sample)
	var bad: Dictionary = _native_configuration()
	bad["zoom"] = NAN
	malformed.native_configuration(bad, 110, true)
	malformed.finish_native(120)
	_check(not malformed.native_complete and not malformed.native_failure.is_empty(), "Reject nonfinite configuration explicitly")
	cases += 1
	for reason: String in ["quit", "overload"]:
		var interrupted: NativeClockMetrics = _native_recorder("native-interrupted-" + reason, sample)
		interrupted.native_interruption(reason, 110)
		interrupted.finish_native(120)
		_check(not interrupted.native_complete and not interrupted.native_failure.is_empty(), "Quit and overload cannot stand in for real input pause")
	cases += 1
	_check(cases == 14, "Complete bounded native boundary cases")
	print("BGR_NATIVE_BOUNDARY_OK cases=" + str(cases))
