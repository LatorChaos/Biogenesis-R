extends SceneTree

const StatisticsWindow = preload("res://ui/statistics_window.gd")
const LANGUAGES: Array[String] = ["en", "es", "ca", "de", "pt", "ru"]
# T_COPY values captured from the original Java property files
const COPY_CAPTIONS: Array[String] = ["Copy", "Copiar", "Copia", "Kopieren", "Copiar", "Копировать"]
const COPY_HINTS: Array[String] = [
	"Right-click or press Enter to copy this genotype.",
	"Haga clic derecho o pulse Intro para copiar este genotipo.",
	"Feu clic dret o premeu Retorn per copiar aquest genotip.",
	"Rechtsklick oder Eingabetaste: diesen Genotyp kopieren.",
	"Clique com o botão direito ou prima Enter para copiar este genótipo.",
	"Нажмите правую кнопку мыши или Enter, чтобы скопировать этот генотип."
]

var _checks: int = 0
var _failures: Array[String] = []
var _observed_language: String = ""
var _active_window: StatisticsWindow


func _initialize() -> void:
	await process_frame
	for language_index: int in range(LANGUAGES.size()):
		var language: String = LANGUAGES[language_index]
		var simulation: BgrSimulation = BgrSimulation.new()
		if not _apply_locale(simulation, language_index):
			_finish()
			return
		var configured: Dictionary = simulation.configure(827364, 1200, 800, 64)
		if not _expect(bool(configured.get("ok", false)), language + ": configure failed"):
			_finish()
			return
		var packet: Dictionary = simulation.statistics_open()
		if not _expect(bool(packet.get("ok", false)) and bool(packet.get("open", false)) and packet.get("language") == language, language + ": native Statistics locale differs"):
			_finish()
			return
		var window: StatisticsWindow = StatisticsWindow.new()
		_active_window = window
		window.simulation = simulation
		window.initial_packet = packet
		root.add_child(window)
		await process_frame
		if not _expect(window._previews.size() == 6, language + ": record preview count differs"):
			_finish()
			return
		_check_previews(window, language_index, "initial")
		if not _apply_locale(simulation, (language_index + 1) % LANGUAGES.size()):
			_finish()
			return
		await process_frame
		_check_previews(window, language_index, "retained")
		window.close_statistics()
		_active_window = null
		await process_frame
		_expect(not bool(simulation.statistics_previews().get("open", true)), language + ": Statistics owner remained open")
	_finish()


func _apply_locale(simulation: BgrSimulation, index: int) -> bool:
	var draft: BgrSettingsDraft = simulation.settings_open()
	var snapshot: Dictionary = draft.snapshot()
	var controls: Dictionary = {
		"texts": snapshot["texts"], "locale": index,
		"automatic_backup": snapshot["automatic_backup"], "hardware": snapshot["hardware"],
		"disable_framebuffer": snapshot["disable_framebuffer"], "automatic_backup_mode": snapshot["automatic_backup_mode"]
	}
	var replaced: Dictionary = draft.replace_controls(controls)
	if not _expect(bool(replaced.get("ok", false)), LANGUAGES[index] + ": locale draft failed: " + str(replaced.get("error", ""))):
		return false
	var expected_callback: String = "" if int(snapshot["locale"]) == index else LANGUAGES[index]
	_observed_language = ""
	var applied: Dictionary = simulation.settings_apply(draft, _observe_locale)
	if not _expect(bool(applied.get("ok", false)), LANGUAGES[index] + ": locale apply failed"):
		return false
	return _expect(_observed_language == expected_callback and applied.get("language") == LANGUAGES[index], LANGUAGES[index] + ": locale callback or applied language differs")


func _observe_locale(packet: Dictionary) -> void:
	_observed_language = str(packet.get("language", ""))


func _check_previews(window: StatisticsWindow, index: int, stage: String) -> void:
	for preview_index: int in range(6):
		var preview: Control = window._previews[preview_index]
		var popup: PopupMenu = preview.get_node("PreviewCopyPopup") as PopupMenu
		var context: String = "%s/%s/%d" % [LANGUAGES[index], stage, preview_index]
		_expect(popup != null and popup.get_item_text(0) == COPY_CAPTIONS[index], context + ": Copy caption differs")
		_expect(preview.tooltip_text == COPY_HINTS[index], context + ": Copy hint differs")


func _expect(condition: bool, message: String) -> bool:
	_checks += 1
	if not condition:
		_failures.append(message)
	return condition


func _finish() -> void:
	if is_instance_valid(_active_window):
		_active_window.close_statistics()
		_active_window = null
	print("BGR_STATISTICS_COPY_LOCALE_CHECK " + JSON.stringify({"result": "PASS" if _failures.is_empty() else "FAIL", "checks": _checks, "failures": _failures}))
	quit(0 if _failures.is_empty() else 1)
