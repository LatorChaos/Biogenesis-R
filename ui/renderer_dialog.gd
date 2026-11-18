extends ConfirmationDialog

signal startup_ready(allowed: bool)
signal status_changed

const MainLabels = preload("res://ui/main_labels.gd")
const COMPATIBILITY: String = "gl_compatibility"
const MOBILE: String = "mobile"
const TRIAL_SECONDS: float = 15.0

var simulation: BgrSimulation
var language: String = "en"
var _choice: OptionButton
var _information: Label
var _description: Label
var _error: Label
var _timer: Timer
var _mode: StringName = &"choose"
var _startup_waiting: bool = false
var _trial_active: bool = false
var _last_error: String = ""
var _notice_key: String = ""


func _ready() -> void:
	name = "RendererDialog"
	force_native = true
	dialog_hide_on_ok = false
	dialog_autowrap = true
	get_label().hide()
	var scroll: ScrollContainer = ScrollContainer.new()
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	scroll.custom_minimum_size = Vector2(500, 210)
	scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	add_child(scroll)
	var column: VBoxContainer = VBoxContainer.new()
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column.add_theme_constant_override("separation", 12)
	scroll.add_child(column)
	_description = Label.new()
	_description.name = "RendererDescription"
	_description.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	column.add_child(_description)
	_information = Label.new()
	_information.name = "RendererInformation"
	_information.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	column.add_child(_information)
	_choice = OptionButton.new()
	_choice.name = "RendererChoice"
	_choice.add_item("", 0)
	_choice.add_item("", 1)
	column.add_child(_choice)
	_error = Label.new()
	_error.name = "RendererError"
	_error.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	column.add_child(_error)
	_timer = Timer.new()
	_timer.one_shot = true
	_timer.wait_time = TRIAL_SECONDS
	_timer.timeout.connect(_reject_trial)
	add_child(_timer)
	confirmed.connect(_confirm)
	canceled.connect(_cancel)
	apply_locale(language)


func apply_locale(value: String) -> void:
	language = value
	title = _message("R_RENDERER_TITLE")
	_choice.set_item_text(0, _message("R_RENDERER_COMPATIBILITY"))
	_choice.set_item_text(1, _message("R_RENDERER_MOBILE"))
	_choice.accessibility_name = title
	_information.text = _message("R_RENDERER_INFORMATION") % [RenderingServer.get_current_rendering_method(), RenderingServer.get_video_adapter_name()]
	_description.text = _message("R_RENDERER_TRIAL") % int(TRIAL_SECONDS) if _mode == &"trial" else _message("R_RENDERER_HELP")
	get_ok_button().text = _message("T_YES") if _mode == &"trial" else (_message("R_LAUNCHER_RESULT_OK") if _mode == &"failure" else _message("R_RENDERER_APPLY"))
	get_cancel_button().text = _message("T_NO") if _mode == &"trial" else _message("T_CANCEL")
	_error.visible = not _last_error.is_empty() or not _notice_key.is_empty()
	_error.text = _message("R_RENDERER_ERROR") % _last_error if not _last_error.is_empty() else (_message(_notice_key) if not _notice_key.is_empty() else "")
	status_changed.emit()


func is_busy() -> bool:
	return visible or _startup_waiting


func begin_startup(safe: bool) -> bool:
	var status: Dictionary = simulation.renderer_status()
	if not bool(status["ok"]):
		_last_error = str(status["error"])
		status_changed.emit()
		return true
	if not _interactive() or (str(status["state"]) == "compatibility" and not safe):
		return false
	_startup_waiting = true
	_continue_startup.call_deferred(safe)
	return true


func _continue_startup(safe: bool) -> void:
	await RenderingServer.frame_post_draw
	var result: Dictionary = simulation.renderer_startup(RenderingServer.get_current_rendering_method(), true, safe)
	if bool(result.get("recovered", false)):
		_notice_key = "R_RENDERER_RECOVERED"
	_apply(result)


func open_settings() -> void:
	if _startup_waiting:
		return
	_mode = &"choose"
	_trial_active = false
	var status: Dictionary = simulation.renderer_status()
	_last_error = str(status["error"])
	var blocked: bool = bool(status.get("busy", true)) or not _interactive() or not bool(status["ok"])
	if bool(status.get("busy", false)):
		_notice_key = "R_RENDERER_BUSY"
	elif not _interactive():
		_notice_key = "R_RENDERER_UNAVAILABLE"
	_choice.select(1 if str(status["target"]) == MOBILE else 0)
	_choice.visible = true
	_choice.disabled = blocked
	get_ok_button().disabled = blocked
	apply_locale(language)
	popup_centered_clamped(Vector2i(620, 300))
	get_cancel_button().grab_focus()


func _confirm() -> void:
	if _mode == &"failure":
		_cancel()
	elif _mode == &"trial":
		_timer.stop()
		_apply(simulation.renderer_confirm(true, RenderingServer.get_current_rendering_method()))
	else:
		var target: String = MOBILE if _choice.selected == 1 else COMPATIBILITY
		_apply(simulation.renderer_request(target, RenderingServer.get_current_rendering_method()))


func _apply(result: Dictionary) -> void:
	if not bool(result["ok"]):
		_last_error = str(result["error"])
		_mode = &"failure"
		_choice.visible = false
		_timer.stop()
		get_ok_button().disabled = false
		apply_locale(language)
		popup_centered_clamped(Vector2i(620, 300))
		get_cancel_button().grab_focus()
		return
	_last_error = ""
	match str(result["action"]):
		"restart":
			_restart(str(result["target"]))
		"confirm":
			_mode = &"trial"
			_trial_active = true
			_choice.visible = false
			get_ok_button().disabled = false
			apply_locale(language)
			popup_centered_clamped(Vector2i(620, 300))
			get_cancel_button().grab_focus()
			_timer.start()
		"continue":
			_finish(true)
		_:
			assert(false, "Unsupported renderer action")


func _cancel() -> void:
	if _trial_active:
		_reject_trial()
	else:
		_finish(false)


func _reject_trial() -> void:
	if not _trial_active:
		return
	_timer.stop()
	var result: Dictionary = simulation.renderer_confirm(false, RenderingServer.get_current_rendering_method())
	if not bool(result["ok"]):
		# Safe startup also suppresses saved requests when preference writes fail
		_restart(COMPATIBILITY)
	else:
		_apply(result)


func _finish(allowed: bool) -> void:
	_timer.stop()
	_trial_active = false
	var waiting: bool = _startup_waiting
	_startup_waiting = false
	status_changed.emit()
	hide()
	if waiting:
		startup_ready.emit(allowed)


func _restart(target: String) -> void:
	assert(target in [COMPATIBILITY, MOBILE])
	assert(not bool(simulation.preferences_snapshot()["session_ready"]))
	OS.set_restart_on_exit(true, restart_arguments(target))
	get_tree().quit()


static func restart_arguments(target: String) -> PackedStringArray:
	assert(target in [COMPATIBILITY, MOBILE])
	# Godot removes consumed engine flags from get_cmdline_args
	var arguments: PackedStringArray = PackedStringArray([
		"--path", ProjectSettings.globalize_path("res://"),
		"--rendering-method", target,
		"--display-driver", DisplayServer.get_name().to_lower(),
		"--audio-driver", AudioServer.get_driver_name(),
		"--max-fps", str(Engine.max_fps)
	])
	arguments.append_array(OS.get_cmdline_args())
	arguments.append("--")
	for argument: String in OS.get_cmdline_user_args():
		if argument != "--safe-graphics":
			arguments.append(argument)
	if target == COMPATIBILITY:
		arguments.append("--safe-graphics")
	return arguments


func _interactive() -> bool:
	return DisplayServer.get_name() in ["X11", "Wayland", "Windows", "macOS"]


func _message(key: String) -> String:
	return MainLabels.message(key, language)


func notice_text() -> String:
	if not _last_error.is_empty():
		return _message("R_RENDERER_ERROR") % _last_error
	return _message(_notice_key) if not _notice_key.is_empty() else ""


func refresh_preferences() -> void:
	if not is_busy():
		_last_error = str(simulation.renderer_status()["error"])
