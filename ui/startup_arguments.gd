extends RefCounted

const USAGE: String = "Usage: Biogenesis-R -- [--launcher | --resume-managed] [random seed]"
# Long.parseLong uses Character.digit(char, 10), including BMP decimal digits
const DECIMAL_ZEROS: Array[int] = [
	0x0030, 0x0660, 0x06f0, 0x07c0, 0x0966, 0x09e6, 0x0a66, 0x0ae6,
	0x0b66, 0x0be6, 0x0c66, 0x0ce6, 0x0d66, 0x0de6, 0x0e50, 0x0ed0,
	0x0f20, 0x1040, 0x1090, 0x17e0, 0x1810, 0x1946, 0x19d0, 0x1a80,
	0x1a90, 0x1b50, 0x1bb0, 0x1c40, 0x1c50, 0xa620, 0xa8d0, 0xa900,
	0xa9d0, 0xa9f0, 0xaa50, 0xabf0, 0xff10
]


static func parse(arguments: PackedStringArray) -> Dictionary:
	var payload: PackedStringArray = arguments.duplicate()
	var resume_managed: bool = not payload.is_empty() and payload[0] == "--resume-managed"
	if resume_managed:
		payload.remove_at(0)
	var launcher_only: bool = not payload.is_empty() and payload[0] == "--launcher"
	if launcher_only:
		payload.remove_at(0)
	var result: Dictionary = {"launcher_only": launcher_only, "valid": true, "seed_text": "", "resume_managed": resume_managed}
	if payload.size() > 1:
		result["valid"] = false
	elif payload.size() == 1:
		var seed: Dictionary = signed_long(payload[0])
		result["valid"] = bool(seed["ok"])
		if bool(seed["ok"]):
			result["seed_text"] = str(seed["value"])
	for argument: String in arguments:
		if argument.begins_with("--resume-managed") and (argument != "--resume-managed" or not resume_managed or launcher_only or not bool(result["valid"])):
			result["valid"] = false
			result["launcher_only"] = true
			result["resume_managed"] = false
	return result


static func signed_long(text: String) -> Dictionary:
	var negative: bool = text.begins_with("-")
	var first: int = 1 if negative or text.begins_with("+") else 0
	if first == text.length():
		return {"ok": false}
	var normalized: String = ""
	for index: int in range(first, text.length()):
		var code: int = text.unicode_at(index)
		var digit: int = -1
		for zero: int in DECIMAL_ZEROS:
			if code >= zero and code < zero + 10:
				digit = code - zero
				break
		if digit < 0:
			return {"ok": false}
		if digit == 0 and normalized.is_empty():
			continue
		if normalized.length() == 19:
			return {"ok": false}
		normalized += str(digit)
	if normalized.is_empty():
		return {"ok": true, "value": 0}
	var limit: String = "9223372036854775808" if negative else "9223372036854775807"
	if normalized.length() == limit.length() and normalized > limit:
		return {"ok": false}
	if negative:
		normalized = "-" + normalized
	return {"ok": true, "value": normalized.to_int()}
