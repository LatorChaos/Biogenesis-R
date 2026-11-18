extends SceneTree

const StartupArguments = preload("res://ui/startup_arguments.gd")


func _initialize() -> void:
	var cases: Array[Array] = [
		["0", true, "0"], ["+00042", true, "42"], ["-0", true, "0"],
		["9223372036854775807", true, "9223372036854775807"],
		["-9223372036854775808", true, "-9223372036854775808"],
		["9223372036854775808", false, ""], ["-9223372036854775809", false, ""],
		["", false, ""], ["+", false, ""], ["--1", false, ""],
		[" 1", false, ""], ["1 ", false, ""], ["1.0", false, ""], ["1e2", false, ""],
		["\u0661\uff12", true, "12"], ["\u00b2", false, ""],
		[String.chr(0x104a1), false, ""],
		["+" + "0".repeat(8192) + "42", true, "42"],
		["1".repeat(8192), false, ""]
	]
	for case: Array in cases:
		var result: Dictionary = StartupArguments.signed_long(str(case[0]))
		if bool(result["ok"]) != bool(case[1]) or (bool(case[1]) and str(result["value"]) != str(case[2])):
			printerr("BGR_GUI_FAIL Signed seed parser: " + str(case[0]))
			quit(1)
			return
	for arguments: PackedStringArray in [PackedStringArray(), PackedStringArray(["--launcher"]), PackedStringArray(["--launcher", "1"]), PackedStringArray(["1", "2"]), PackedStringArray(["--launcher", "1", "2"])]:
		var result: Dictionary = StartupArguments.parse(arguments)
		if bool(result["valid"]) != (arguments.size() < (3 if arguments.has("--launcher") else 2)):
			printerr("BGR_GUI_FAIL Startup argument count")
			quit(1)
			return
	print("BGR_LIFECYCLE_SEED_PASS 24")
	quit(0)
