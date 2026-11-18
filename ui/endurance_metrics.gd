extends RefCounted

const OPTION_PREFIX: String = "--bgr-endurance-"
const DIRECTORY_OPTION: String = "--bgr-endurance-dir="
const NATIVE_OPTION: String = "--bgr-endurance-native"
const NATIVE_ROWS: int = 4096
const NATIVE_COLUMNS: int = 9
const NATIVE_EVENTS: int = 64
const NATIVE_BYTES: int = 1048576
const NATIVE_DURATION_USEC: int = 110000000
const NATIVE_FREEZE_USEC: int = 300000
const INTERVAL_USEC: int = 60_000_000
const MAX_DURATION_USEC: int = 87_000_000_000
const MAX_ROWS: int = 1452
const MAX_ROW_BYTES: int = 8192
const MAX_BYTES: int = 12 * 1024 * 1024

var active: bool = false
var failure: String = ""
var rows_written: int = 0
var bytes_written: int = 0
var _file: FileAccess
var _started_usec: int = 0
var _interval_started_usec: int = 0
var _next_usec: int = 0
var _last_dispatch_usec: int = -1
var _dispatch_count: int = 0
var _gap_count: int = 0
var _gap_sum_usec: int = 0
var _gap_max_usec: int = 0
var _batch_count: int = 0
var _batch_ticks: int = 0
var _batch_failed_count: int = 0
var _batch_max_usec: int = 0
var _debt_max: float = 0.0
var _paused_seen: bool = false
var _runtime_failure_seen: bool = false
var _directory: String = ""
var native_enabled: bool = false
var native_active: bool = false
var native_complete: bool = false
var native_failure: String = ""
var native_count: int = 0
var _native_rows: PackedInt64Array = PackedInt64Array()
var _native_frames: FileAccess
var _native_events: FileAccess
var _native_bytes: int = 0
var _native_event_count: int = 0
var _native_started: int = 0
var _native_segment: int = 0
var _native_last_frame: int = -1
var _native_last_usec: int = -1
var _native_last_tick: int = -1
var _native_last_paused: bool = true
var _native_ran: bool = false
var _native_reached_80: bool = false
var _native_request: int = 0
var _native_requested_paused: bool = true
var _native_requested_tick: int = 0
var _native_run_tick: int = -1
var _native_last_event_usec: int = -1
var _native_pending_pause: int = 0
var _native_pause_tick: int = -1
var _native_observed_usec: int = -1
var _native_dump_queued: bool = false
var _native_dumped: bool = false
var _native_events_healthy: bool = true
var _native_frames_healthy: bool = true


static func options(arguments: PackedStringArray) -> Dictionary:
	var directory: String = ""
	var count: int = 0
	var native_count: int = 0
	for argument: String in arguments:
		if not argument.begins_with(OPTION_PREFIX):
			continue
		if argument == NATIVE_OPTION:
			native_count += 1
			continue
		count += 1
		if not argument.begins_with(DIRECTORY_OPTION):
			return {"enabled": false, "error": "Unsupported endurance option"}
		directory = argument.trim_prefix(DIRECTORY_OPTION)
	if native_count > 1 or (native_count == 1 and count != 1):
		return {"enabled": false, "error": "Native capture requires one endurance directory"}
	if count == 0:
		return {"enabled": false, "error": ""}
	if count != 1 or _contains_nul_codepoint(directory) or not directory.begins_with("/") or directory != directory.simplify_path():
		return {"enabled": false, "error": "Require one canonical absolute endurance directory"}
	return {"enabled": true, "error": "", "directory": directory, "native": native_count == 1}


static func _contains_nul_codepoint(text: String) -> bool:
	for index: int in range(text.length()):
		if text.unicode_at(index) == 0:
			return true
	return false


static func gameplay_arguments(arguments: PackedStringArray) -> PackedStringArray:
	var result: PackedStringArray = PackedStringArray()
	for argument: String in arguments:
		if not argument.begins_with(OPTION_PREFIX):
			result.append(argument)
	return result


func start(directory: String, now_usec: int, sample: Dictionary) -> bool:
	if _file != null or active or rows_written != 0 or not failure.is_empty():
		_fail("Recorder start was repeated", now_usec)
		return false
	if now_usec < 0 or _contains_nul_codepoint(directory) or not directory.begins_with("/") or directory != directory.simplify_path():
		_fail("Invalid recorder start", now_usec)
		return false
	if not _valid_sample(sample):
		_fail("Invalid cached application sample", now_usec)
		return false
	# A fresh child directory reserves the stream under the caller-owned parent
	if DirAccess.make_dir_absolute(directory) != OK:
		_fail("Cannot reserve fresh endurance directory", now_usec)
		return false
	var path: String = directory.path_join("application.jsonl")
	if FileAccess.file_exists(path) or DirAccess.dir_exists_absolute(path):
		_fail("Endurance output already exists", now_usec)
		return false
	_file = FileAccess.open(path, FileAccess.WRITE)
	if _file == null:
		_fail("Cannot open endurance output", now_usec)
		return false
	_started_usec = now_usec
	_directory = directory
	_interval_started_usec = now_usec
	_next_usec = now_usec + INTERVAL_USEC
	active = true
	return _append(_row("start", now_usec, sample), now_usec)


func observe_dispatch(now_usec: int, paused: bool, runtime_failed: bool) -> void:
	if not active:
		return
	if now_usec < _started_usec or (_last_dispatch_usec >= 0 and now_usec < _last_dispatch_usec):
		_fail("Monotonic dispatch clock moved backwards", now_usec)
		return
	_dispatch_count += 1
	if _last_dispatch_usec >= 0:
		var gap: int = now_usec - _last_dispatch_usec
		_gap_count += 1
		_gap_sum_usec += gap
		_gap_max_usec = maxi(_gap_max_usec, gap)
	_last_dispatch_usec = now_usec
	_paused_seen = _paused_seen or paused
	_runtime_failure_seen = _runtime_failure_seen or runtime_failed


func observe_debt(debt: float) -> void:
	if not active:
		return
	if not is_finite(debt) or debt < 0.0:
		_fail("Invalid pending biological tick debt", _last_dispatch_usec)
		return
	_debt_max = maxf(_debt_max, debt)


func observe_batch(ticks: int, elapsed_usec: int, accepted: bool) -> void:
	if not active:
		return
	if ticks < 0 or elapsed_usec < 0:
		_fail("Invalid advance-and-accept batch", _last_dispatch_usec)
		return
	_batch_count += 1
	_batch_ticks += ticks
	_batch_max_usec = maxi(_batch_max_usec, elapsed_usec)
	if not accepted:
		_batch_failed_count += 1


func due(now_usec: int) -> bool:
	return active and now_usec >= _next_usec


func record(now_usec: int, sample: Dictionary) -> void:
	if not active:
		return
	if now_usec - _started_usec >= MAX_DURATION_USEC:
		finish(now_usec, sample, "time_limit")
		return
	if not due(now_usec):
		return
	if _append(_row("interval", now_usec, sample), now_usec):
		_reset_interval(now_usec)
		_next_usec = _started_usec + (floori(float(now_usec - _started_usec) / INTERVAL_USEC) + 1) * INTERVAL_USEC


func finish(now_usec: int, sample: Dictionary, reason: String) -> void:
	if not active:
		return
	if reason not in ["shutdown", "time_limit"]:
		_fail("Invalid recorder finalization reason", now_usec)
		return
	var row: Dictionary = _row("final", now_usec, sample)
	row["reason"] = reason
	row["collection_ok"] = true
	if _append(row, now_usec, false):
		active = false
		_file.close()
		_file = null


func _row(kind: String, now_usec: int, sample: Dictionary) -> Dictionary:
	return {
		"schema": 1, "kind": kind, "monotonic_usec": now_usec,
		"utc_unix_seconds": Time.get_unix_time_from_system(),
		"elapsed_usec": now_usec - _started_usec,
		"interval_started_usec": _interval_started_usec, "interval_ended_usec": now_usec,
		"missed_minute_boundaries": maxi(0, floori(float(now_usec - _next_usec) / INTERVAL_USEC)),
		"dispatch_count": _dispatch_count, "dispatch_gap_count": _gap_count,
		"dispatch_gap_sum_usec": _gap_sum_usec, "dispatch_gap_max_usec": _gap_max_usec,
		"dispatch_gap_mean_usec": float(_gap_sum_usec) / _gap_count if _gap_count > 0 else null,
		"advance_and_accept_batch_count": _batch_count,
		"advance_and_accept_completed_ticks": _batch_ticks,
		"advance_and_accept_failed_count": _batch_failed_count,
		"advance_and_accept_batch_max_usec": _batch_max_usec,
		"pending_ticks_max": _debt_max, "paused_seen": _paused_seen,
		"runtime_failure_seen": _runtime_failure_seen, "application": sample
	}


func _valid_sample(sample: Dictionary) -> bool:
	for key: String in ["tick", "world_time", "population", "seed", "width", "height", "initial_attempts", "configured_delay_ms", "effective_delay_ms"]:
		if not sample.has(key) or typeof(sample[key]) != TYPE_INT:
			return false
	for key: String in ["paused", "runtime_failed", "crisp_lines", "state_available"]:
		if not sample.has(key) or typeof(sample[key]) != TYPE_BOOL:
			return false
	if typeof(sample.get("pending_ticks")) != TYPE_FLOAT or not is_finite(sample["pending_ticks"]) or float(sample["pending_ticks"]) < 0.0:
		return false
	if int(sample["effective_delay_ms"]) < 1 or int(sample["width"]) < 0 or int(sample["height"]) < 0:
		return false
	if typeof(sample.get("network")) != TYPE_DICTIONARY:
		return false
	var network: Dictionary = sample["network"]
	for key: String in ["pending_command_jobs", "connections", "queued_incoming", "incoming_candidates", "pending_outgoing", "job_limit", "incoming_limit"]:
		if typeof(network.get(key)) != TYPE_INT or int(network[key]) < 0:
			return false
	return typeof(network.get("closing")) == TYPE_BOOL


func _append(row: Dictionary, now_usec: int, reserve_terminal: bool = true) -> bool:
	if now_usec < _interval_started_usec or (_last_dispatch_usec >= 0 and now_usec < _last_dispatch_usec):
		_fail("Monotonic row clock moved backwards", now_usec)
		return false
	if not _valid_sample(row["application"]):
		_fail("Invalid cached application sample", now_usec)
		return false
	var data: PackedByteArray = (JSON.stringify(row) + "\n").to_utf8_buffer()
	var reserve_rows: int = 1 if reserve_terminal else 0
	var reserve_bytes: int = MAX_ROW_BYTES if reserve_terminal else 0
	if data.size() > MAX_ROW_BYTES or rows_written + 1 + reserve_rows > MAX_ROWS or bytes_written + data.size() + reserve_bytes > MAX_BYTES:
		_fail("Endurance row or output quota exceeded", now_usec)
		return false
	if not _write(data):
		_fail("Endurance output write or flush failed", now_usec, false)
		return false
	return true


func _write(data: PackedByteArray) -> bool:
	if _file == null or not _file.is_open():
		return false
	_file.store_buffer(data)
	if _file.get_error() != OK:
		return false
	_file.flush()
	if _file.get_error() != OK:
		return false
	rows_written += 1
	bytes_written += data.size()
	return true


func _fail(reason: String, now_usec: int, stream_healthy: bool = true) -> void:
	if not failure.is_empty():
		return
	failure = reason
	active = false
	if not _native_rows.is_empty():
		_native_fail("Minute recorder failed: " + reason, now_usec)
	printerr("BGR_ENDURANCE_FAILURE " + reason)
	if _file != null and stream_healthy:
		var data: PackedByteArray = (JSON.stringify({
			"schema": 1, "kind": "failure", "collection_ok": false, "reason": reason,
			"monotonic_usec": now_usec, "utc_unix_seconds": Time.get_unix_time_from_system(),
			"confirmed_rows_before_failure": rows_written, "confirmed_bytes_before_failure": bytes_written
		}) + "\n").to_utf8_buffer()
		if data.size() <= MAX_ROW_BYTES and rows_written < MAX_ROWS and bytes_written + data.size() <= MAX_BYTES:
			_write(data)
	if _file != null:
		if _file.is_open():
			_file.close()
		_file = null


func _reset_interval(now_usec: int) -> void:
	_interval_started_usec = now_usec
	_dispatch_count = 0
	_gap_count = 0
	_gap_sum_usec = 0
	_gap_max_usec = 0
	_batch_count = 0
	_batch_ticks = 0
	_batch_failed_count = 0
	_batch_max_usec = 0
	_debt_max = 0.0
	_paused_seen = false
	_runtime_failure_seen = false


func start_native(directory: String, now_usec: int) -> bool:
	if not active or native_enabled or not native_failure.is_empty() or directory != _directory or now_usec < _started_usec:
		_native_fail("Invalid native capture start", now_usec)
		return false
	native_enabled = true
	for name: String in ["native-frames.tsv", "native-events.jsonl"]:
		var path: String = directory.path_join(name)
		if FileAccess.file_exists(path) or DirAccess.dir_exists_absolute(path):
			_native_fail("Native output already exists", now_usec)
			return false
	_native_frames = FileAccess.open(directory.path_join("native-frames.tsv"), FileAccess.WRITE)
	_native_events = FileAccess.open(directory.path_join("native-events.jsonl"), FileAccess.WRITE)
	if _native_frames == null or _native_events == null:
		_native_fail("Cannot open native output", now_usec)
		return false
	_native_rows.resize(NATIVE_ROWS * NATIVE_COLUMNS)
	_native_started = now_usec
	native_active = true
	return true


func native_configuration(configuration: Dictionary, now_usec: int, paused: bool) -> void:
	if not native_collecting():
		return
	if not _native_valid_configuration(configuration):
		_native_fail("Invalid native configuration", now_usec)
		return
	if _native_segment > 0 and (not paused or _native_ran or _native_pending_pause != 0):
		_native_fail("Configuration changed after native run admission", now_usec)
		return
	_native_segment += 1
	_native_last_tick = -1
	_native_event("configuration", now_usec, {"segment": _native_segment, "application": configuration}, true)


func _native_valid_configuration(configuration: Dictionary) -> bool:
	for key: String in ["seed", "width", "height", "initial_attempts", "window_id", "tick", "max_fps", "thread_setting", "vsync_mode"]:
		if typeof(configuration.get(key)) != TYPE_INT:
			return false
	if int(configuration["width"]) <= 0 or int(configuration["height"]) <= 0 or int(configuration["initial_attempts"]) < 0:
		return false
	for key: String in ["paused", "state_available", "fit_enabled", "crisp_lines", "thread_setting_present"]:
		if typeof(configuration.get(key)) != TYPE_BOOL:
			return false
	if not bool(configuration["thread_setting_present"]) and (int(configuration["thread_setting"]) != -1 or configuration.get("thread_setting_status") != "UNKNOWN"):
		return false
	if not bool(configuration["state_available"]) or configuration.get("screen_refresh_hz") != null or configuration.get("screen_refresh_status") != "UNKNOWN":
		return false
	for key: String in ["window_size", "window_position", "canvas_rect", "offset"]:
		if not configuration.get(key) is Array or configuration[key].size() != (4 if key == "canvas_rect" else 2):
			return false
		for value: Variant in configuration[key]:
			if typeof(value) not in [TYPE_INT, TYPE_FLOAT] or not is_finite(float(value)):
				return false
	if typeof(configuration.get("zoom")) != TYPE_FLOAT or not is_finite(configuration["zoom"]) or float(configuration["zoom"]) <= 0.0:
		return false
	if not configuration.get("controls") is Dictionary:
		return false
	for value: Variant in configuration["controls"].values():
		if not value is Array or value.size() != 4:
			return false
		for coordinate: Variant in value:
			if typeof(coordinate) not in [TYPE_INT, TYPE_FLOAT] or not is_finite(float(coordinate)):
				return false
	return true


func native_request(source: String, now_usec: int, tick: int, paused: bool) -> int:
	if not native_active:
		return 0
	if source not in ["space", "p", "toolbar", "menu", "toolbar_mnemonic"] or _native_pending_pause != 0:
		_native_fail("Invalid or overlapping native pause request", now_usec)
		return 0
	_native_request += 1
	_native_requested_paused = paused
	_native_requested_tick = tick
	_native_event("request", now_usec, {"request": _native_request, "source": source,
		"tick": tick, "from_paused": paused})
	return _native_request if native_active else 0


func native_command(request: int, now_usec: int, accepted: bool, tick: int, paused: bool) -> void:
	if not native_active:
		return
	if request <= 0 or request != _native_request:
		_native_fail("Native command request identity differs", now_usec)
		return
	if not _native_event("accepted" if accepted else "rejected", now_usec,
			{"request": request, "tick": tick, "to_paused": paused}):
		return
	if not accepted:
		_native_fail("Native pause command was rejected", now_usec)
	elif paused == _native_requested_paused or tick != _native_requested_tick:
		_native_fail("Native command state acknowledgement differs", now_usec)
	elif paused:
		_native_pending_pause = request
		_native_pause_tick = tick
	elif _native_run_tick < 0:
		_native_run_tick = tick


func native_interruption(source: String, now_usec: int) -> void:
	if native_collecting():
		_native_event("interruption", now_usec, {"source": source})
		_native_fail("Non-input pause or world decision: " + source, now_usec)


func native_phase(frame: int, now_usec: int, tick: int, paused: bool, runtime_failed: bool) -> void:
	if not native_active:
		return
	if native_count >= NATIVE_ROWS or now_usec < _native_started or now_usec - _native_started > NATIVE_DURATION_USEC:
		_native_fail("Native row or duration quota exceeded", now_usec)
		return
	if frame <= _native_last_frame or now_usec < _native_last_usec or tick < _native_last_tick or tick < 0 or _native_segment == 0:
		_native_fail("Native frame, clock, tick or segment order differs", now_usec)
		return
	var offset: int = native_count * NATIVE_COLUMNS
	_native_rows[offset] = native_count
	_native_rows[offset + 1] = frame
	_native_rows[offset + 2] = now_usec
	_native_rows[offset + 3] = tick
	_native_rows[offset + 4] = 1 if paused else 0
	_native_rows[offset + 5] = 1 if runtime_failed else 0
	_native_rows[offset + 6] = _native_segment
	_native_rows[offset + 7] = _native_pending_pause
	native_count += 1
	_native_last_frame = frame
	_native_last_usec = now_usec
	_native_last_tick = tick
	if runtime_failed:
		_native_fail("Native sample reports application failure", now_usec)
	elif paused and not _native_last_paused and _native_pending_pause == 0:
		_native_fail("Paused without an accepted input request", now_usec)
	elif _native_pending_pause != 0:
		if not paused or tick != _native_pause_tick:
			_native_fail("Accepted pause did not retain its tick", now_usec)
		elif not _native_reached_80:
			_native_fail("Native pause preceded reached80", now_usec)
		elif _native_observed_usec < 0:
			_native_observed_usec = now_usec
			_native_event("pause_observed", now_usec, {"request": _native_pending_pause,
				"row": native_count - 1, "frame": frame, "tick": tick})
		elif now_usec - _native_observed_usec >= NATIVE_FREEZE_USEC:
			if _native_event("frozen", now_usec, {"request": _native_pending_pause,
					"row": native_count - 1, "frame": frame, "tick": tick,
					"frozen_usec": now_usec - _native_observed_usec}):
				native_active = false
				_native_queue_dump()
	elif not paused:
		_native_ran = true
		if _native_run_tick >= 0 and tick - _native_run_tick >= 80 and not _native_reached_80:
			_native_reached_80 = true
			_native_event("reached80", now_usec, {"row": native_count - 1, "frame": frame, "tick": tick,
				"completed_ticks": tick - _native_run_tick})
	_native_last_paused = paused
	_native_rows[offset + 8] = _native_clock() - now_usec
	if _native_rows[offset + 8] < 0:
		_native_fail("Native sampling clock moved backwards", now_usec)


func _native_clock() -> int:
	return Time.get_ticks_usec()


func _native_event(kind: String, now_usec: int, fields: Dictionary, configuration: bool = false) -> bool:
	if not _native_events_healthy or _native_events == null:
		return false
	if kind != "failure" and (now_usec < _native_started or now_usec < _native_last_event_usec):
		_native_fail("Native control clock moved backwards", now_usec)
		return false
	var row: Dictionary = {"schema": 1, "event": _native_event_count, "kind": kind, "engine_usec": now_usec}
	row.merge(fields)
	var data: PackedByteArray = (JSON.stringify(row) + "\n").to_utf8_buffer()
	var terminal: bool = kind in ["failure", "complete"]
	if data.size() > (8192 if configuration else 512) or _native_event_count + (1 if terminal else 2) > NATIVE_EVENTS or _native_bytes + data.size() + (0 if terminal else 8192) > NATIVE_BYTES:
		_native_fail("Native control event quota exceeded", now_usec)
		return false
	if not _native_write(_native_events, data, true):
		_native_events_healthy = false
		_native_fail("Native event write or flush unconfirmed", now_usec)
		return false
	_native_event_count += 1
	_native_last_event_usec = now_usec
	return true


func _native_write(file: FileAccess, data: PackedByteArray, flush: bool) -> bool:
	if file == null or not file.is_open():
		return false
	var before: int = file.get_position()
	file.store_buffer(data)
	if file.get_error() != OK or file.get_position() != before + data.size():
		return false
	if flush:
		file.flush()
		if file.get_error() != OK:
			return false
	_native_bytes += data.size()
	return true


func _native_queue_dump() -> void:
	_native_dump_queued = true


func native_collecting() -> bool:
	return not _native_rows.is_empty() and not native_complete and native_failure.is_empty()


func native_flush_pending() -> void:
	if _native_dump_queued:
		_native_dump()


func _native_dump() -> void:
	if _native_dumped:
		return
	_native_dumped = true
	var spans: PackedInt64Array = PackedInt64Array()
	spans.resize(native_count)
	for index: int in range(native_count):
		spans[index] = _native_rows[index * NATIVE_COLUMNS + 8]
	spans.sort()
	var span_max: int = spans[-1] if not spans.is_empty() else 0
	var span_p95: int = spans[ceili(0.95 * spans.size()) - 1] if not spans.is_empty() else 0
	if native_failure.is_empty() and (span_max > 1000 or span_p95 > 100):
		_native_fail("Native sampler exceeds engineering ceiling", _native_last_usec)
	if _native_frames != null and _native_frames_healthy:
		for index: int in range(native_count):
			var columns: PackedStringArray = PackedStringArray()
			for column: int in range(NATIVE_COLUMNS):
				columns.append(str(_native_rows[index * NATIVE_COLUMNS + column]))
			var data: PackedByteArray = ("\t".join(columns) + "\n").to_utf8_buffer()
			if _native_bytes + data.size() + 8192 > NATIVE_BYTES or not _native_write(_native_frames, data, false):
				_native_frames_healthy = false
				_native_fail("Native frame output quota or write failure", _native_last_usec)
				break
		if _native_frames_healthy:
			_native_frames.flush()
			if _native_frames.get_error() != OK:
				_native_frames_healthy = false
				_native_fail("Native frame flush unconfirmed", _native_last_usec)
	if native_failure.is_empty():
		if native_count == 0 or _native_observed_usec < 0 or _native_last_usec - _native_observed_usec < NATIVE_FREEZE_USEC:
			_native_fail("Native collection lacks causal freeze", _native_last_usec)
		else:
			native_complete = _native_event("complete", _native_clock(), {"collection_ok": true,
				"rows": native_count, "request": _native_pending_pause, "tick": _native_last_tick,
				"span_p95_usec": span_p95, "span_max_usec": span_max})
	for file: FileAccess in [_native_frames, _native_events]:
		if file != null and file.is_open():
			file.close()
	_native_frames = null
	_native_events = null


func _native_fail(reason: String, now_usec: int) -> void:
	if not native_failure.is_empty():
		return
	native_failure = reason
	native_active = false
	native_complete = false
	printerr("BGR_NATIVE_FAILURE " + reason)
	_native_event("failure", now_usec, {"collection_ok": false, "reason": reason,
		"confirmed_rows": native_count, "confirmed_events": _native_event_count})
	_native_queue_dump()


func finish_native(now_usec: int) -> void:
	if not native_enabled:
		return
	if not native_complete and native_failure.is_empty():
		_native_fail("Shutdown before confirmed native completion", now_usec)
	_native_dump()
