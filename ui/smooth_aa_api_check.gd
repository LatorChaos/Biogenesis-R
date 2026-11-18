extends Node

var _failures: Array[String] = []
var _checks: int = 0


func _ready() -> void:
	_run.call_deferred()


func _check(condition: bool, message: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(message)


func _packet(segments: PackedFloat64Array, colors: PackedInt32Array, alphas: PackedByteArray = PackedByteArray(),
		zoom: float = 1.0, offset: Vector2 = Vector2.ZERO, allowed: bool = true, viewport: Vector2 = Vector2(128, 128)) -> Dictionary:
	return BgrSimulation.prepare_smooth_aa_viewport_batch(segments, colors, alphas, zoom, offset, allowed, viewport)


func _run() -> void:
	var empty: Dictionary = _packet(PackedFloat64Array(), PackedInt32Array())
	_check(empty["eligible"] and empty["aa"] and empty["commands"].is_empty(), "Empty packet remains atomic and eligible")
	for setting: Dictionary in [
		{"segments": PackedFloat64Array([0, 0, 1]), "colors": PackedInt32Array([0xffffff])},
		{"segments": PackedFloat64Array([0, 0, 1, 1]), "colors": PackedInt32Array()},
		{"segments": PackedFloat64Array([0, 0, 1, 1]), "colors": PackedInt32Array([0xffffff]), "alphas": PackedByteArray([0, 1])},
		{"segments": PackedFloat64Array([0, 0, NAN, 1]), "colors": PackedInt32Array([0xffffff])},
	]:
		var refused: Dictionary = _packet(setting["segments"], setting["colors"], setting.get("alphas", PackedByteArray()))
		_check(not refused["eligible"] and refused["commands"].is_empty() and not refused.has("aa_points"), "Invalid arrays expose no partial AA output")
	for setting: Dictionary in [{"zoom": 0.0}, {"offset": Vector2(NAN, 0)}, {"viewport": Vector2.ZERO}, {"allowed": false}]:
		var refused: Dictionary = _packet(PackedFloat64Array([0, 0, 1, 1]), PackedInt32Array([0xffffff]), PackedByteArray(),
			setting.get("zoom", 1.0), setting.get("offset", Vector2.ZERO), setting.get("allowed", true), setting.get("viewport", Vector2(128, 128)))
		_check(not refused["eligible"] and refused["commands"].is_empty(), "Invalid transform or eligibility refuses without partial output")
	for segments: PackedFloat64Array in [PackedFloat64Array([0, 0, 1048577, 0]), PackedFloat64Array([0, 0, 1e-30, 0])]:
		var legacy: Dictionary = BgrSimulation.prepare_smooth_viewport_batch(segments, PackedInt32Array([0xffffff]), PackedByteArray(), 1.0, Vector2.ZERO, true, Vector2(128, 128))
		var fallback: Dictionary = _packet(segments, PackedInt32Array([0xffffff]))
		_check(fallback == legacy and not fallback.has("aa_points"), "Unsafe AA domain returns the exact complete legacy packet")
	var segments: PackedFloat64Array = PackedFloat64Array([10, 10, 20, 20, 30, 30, 30, 30, 40, 40, 50, 50])
	var colors: PackedInt32Array = PackedInt32Array([0xff0000, 0x0000ff, 0x00ff00])
	var alphas: PackedByteArray = PackedByteArray([128, 255, 128])
	var input_bytes: Array[PackedByteArray] = [segments.to_byte_array(), colors.to_byte_array(), alphas.duplicate()]
	var ordered: Dictionary = _packet(segments, colors, alphas)
	_check(ordered["commands"] == PackedInt32Array([0, -1, 1]), "Point barriers preserve both contiguous line runs")
	_check(ordered["point_rects"][0] == Rect2(29.5, 29.5, 1, 1) and ordered["point_colors"][0] == Color.BLUE, "Point rectangle and color remain unchanged")
	_check([segments.to_byte_array(), colors.to_byte_array(), alphas] == input_bytes, "Preparation does not mutate input arrays")
	var many: PackedFloat64Array = PackedFloat64Array()
	var many_colors: PackedInt32Array = PackedInt32Array()
	for index: int in range(2049):
		many.append_array(PackedFloat64Array([10, 10, 20, 20]))
		many_colors.append(0xffffff)
	var chunks: Dictionary = _packet(many, many_colors)
	_check(chunks["commands"] == PackedInt32Array([0, 1]) and chunks["aa_points"].size() == 2, "Chunk boundary preserves one ordered run")
	_check(chunks["aa_points"][0].size() == 2048 * 36 and chunks["aa_points"][1].size() == 36, "Native vertex allocation obeys the chunk bound")
	_check(chunks["aa_indices"][0].size() == 2048 * 54 and chunks["aa_indices"][1] == PackedInt32Array([0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12, 14, 15, 16, 17, 18, 16, 18, 19, 20, 21, 22, 20, 22, 23, 24, 25, 26, 24, 26, 27, 28, 29, 30, 28, 30, 31, 32, 33, 34, 32, 34, 35]), "Chunk-local indices retain the original quad diagonals")
	_check(chunks["aa_packed_bytes"] == 2049 * 1080, "Packed allocation accounting matches returned arrays")
	print("BGR_AA_API " + JSON.stringify({"checks": _checks, "failures": _failures, "simulation_calls": 0}))
	get_tree().quit(0 if _failures.is_empty() else 1)
