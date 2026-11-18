# Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
# SPDX-License-Identifier: GPL-2.0-or-later

extends AcceptDialog

signal closed

const MANUAL_DIRECTORY: String = "/usr/share/doc/biogenesis/biogenesis_manual"
const ARTWORK = preload("res://ui/assets/bullet.jpg")
const OPENERS: Array[String] = ["xdg-open", "gio", "gvfs-open", "kde-open5", "kde-open", "exo-open", "x-www-browser", "firefox", "opera", "konqueror", "epiphany", "mozilla", "netscape", "iceweasel"]
const EXECUTE_BITS: int = FileAccess.UNIX_EXECUTE_OWNER | FileAccess.UNIX_EXECUTE_GROUP | FileAccess.UNIX_EXECUTE_OTHER
const DATA: Dictionary = {
	"en": {
		"R_VERSION_START_ERROR": "The version request could not start (engine status %d): %s",
		"R_VERSION_TRANSPORT_ERROR": "The version request failed (transport status %d).",
		"R_VERSION_TIMEOUT": "The version request timed out after %d seconds.",
		"R_VERSION_BODY_LIMIT": "The version response exceeds %d bytes.",
		"R_VERSION_REDIRECT_LIMIT": "The version request exceeded %d redirects.",
		"R_VERSION_HTTP_ERROR": "The version server returned HTTP %d.",
		"R_VERSION_EMPTY": "The version server returned an empty response.",
		"R_VERSION_UTF8": "The version server returned malformed UTF-8 text.",
		"T_CHECK_LAST_VERSION": "Check last version",
		"T_CHECK_LAST_VERSION_MNEMONIC": "V",
		"T_YOUR_VERSION": "Your version: ",
		"T_LAST_VERSION": "Last version: ",
		"T_VERSION_TITLE": "Version",
		"T_CANT_CHECK_CURRENT_VERSION": "I can't check the current version: ",
		"T_HELP": "Help",
		"T_USER_MANUAL": "User Manual",
		"T_ABOUT": "About Biogenesis",
		"T_HELP_MNEMONIC": "H",
		"T_USER_MANUAL_MNEMONIC": "M",
		"T_ABOUT_MNEMONIC": "A",
		"T_PROGRAM_NAME": "Biogenesis\n",
		"T_VERSION_B": "Biogenesis version:",
		"T_VERSION_CM": "Color Mod version:",
		"T_VERSION_BB": "Backup Button version:",
		"T_COPYRIGHT": "\nCopyright (c) Joan Queralt Molina  2006-2010\nColor Mod by MarcoDBAA 2012-2022\nBackup Button by Tyler Coleman 2013\nSTRtrees (by Vivid Solutions) implemented by Richard David Williams 2017\n",
		"T_ARTWORK_BY": "Artwork by"
	},
	"es": {
		"R_VERSION_START_ERROR": "No se pudo iniciar la consulta de versión (código del motor %d): %s",
		"R_VERSION_TRANSPORT_ERROR": "La consulta de versión falló (código de transporte %d).",
		"R_VERSION_TIMEOUT": "La consulta de versión agotó el tiempo de espera de %d segundos.",
		"R_VERSION_BODY_LIMIT": "La respuesta de versión supera %d bytes.",
		"R_VERSION_REDIRECT_LIMIT": "La consulta de versión superó el límite de %d redirecciones.",
		"R_VERSION_HTTP_ERROR": "El servidor de versiones devolvió HTTP %d.",
		"R_VERSION_EMPTY": "El servidor de versiones devolvió una respuesta vacía.",
		"R_VERSION_UTF8": "El servidor de versiones devolvió texto UTF-8 incorrecto.",
		"T_CHECK_LAST_VERSION": "Comprobar última versión",
		"T_CHECK_LAST_VERSION_MNEMONIC": "V",
		"T_YOUR_VERSION": "Su versión:",
		"T_LAST_VERSION": "Última versión:",
		"T_VERSION_TITLE": "Versión",
		"T_CANT_CHECK_CURRENT_VERSION": "No se puede comprobar la versión actual:",
		"T_HELP": "Ayuda",
		"T_USER_MANUAL": "Manual de usuario",
		"T_ABOUT": "Acerca de Biogénesis",
		"T_HELP_MNEMONIC": "A",
		"T_USER_MANUAL_MNEMONIC": "M",
		"T_ABOUT_MNEMONIC": "S",
		"T_PROGRAM_NAME": "Biogenesis\n",
		"T_VERSION_B": "Biogenesis versión:",
		"T_VERSION_CM": "Color Mod versión:",
		"T_VERSION_BB": "Backup Button versión:",
		"T_COPYRIGHT": "\nCopyright (c) Joan Queralt Molina  2006-2010\nColor Mod por MarcoDBAA 2012-2022\nBackup Button por Tyler Coleman 2013\nSTRtrees (por Vivid Solutions) implementado por Richard David Williams 2017\n",
		"T_ARTWORK_BY": "Gráficos por"
	},
	"ca": {
		"R_VERSION_START_ERROR": "La petició de versió no s'ha pogut iniciar (codi del motor %d): %s",
		"R_VERSION_TRANSPORT_ERROR": "La petició de versió ha fallat (codi de transport %d).",
		"R_VERSION_TIMEOUT": "La petició de versió ha superat el límit de %d segons.",
		"R_VERSION_BODY_LIMIT": "La resposta de versió supera %d bytes.",
		"R_VERSION_REDIRECT_LIMIT": "La petició de versió ha superat el límit de %d redireccions.",
		"R_VERSION_HTTP_ERROR": "El servidor de versions ha retornat HTTP %d.",
		"R_VERSION_EMPTY": "El servidor de versions ha retornat una resposta buida.",
		"R_VERSION_UTF8": "El servidor de versions ha retornat text UTF-8 mal format.",
		"T_CHECK_LAST_VERSION": "Comprova l'última versió",
		"T_CHECK_LAST_VERSION_MNEMONIC": "V",
		"T_YOUR_VERSION": "La vostra versió:",
		"T_LAST_VERSION": "L'última versió:",
		"T_VERSION_TITLE": "Versió",
		"T_CANT_CHECK_CURRENT_VERSION": "No es pot comprovar la versió actual:",
		"T_HELP": "Ajuda",
		"T_USER_MANUAL": "Manual de l'usuari",
		"T_ABOUT": "Quant a Biogènesi",
		"T_HELP_MNEMONIC": "A",
		"T_USER_MANUAL_MNEMONIC": "M",
		"T_ABOUT_MNEMONIC": "Q",
		"T_PROGRAM_NAME": "Biogenesis\n",
		"T_VERSION_B": "Biogenesis versió:",
		"T_VERSION_CM": "Color Mod versió:",
		"T_VERSION_BB": "Backup Button versió:",
		"T_COPYRIGHT": "\nCopyright (c) Joan Queralt Molina  2006-2010\nColor Mod per MarcoDBAA 2012-2022\nBackup Button per Tyler Coleman 2013\nSTRtrees (per Vivid Solutions) implementat per Richard David Williams 2017\n",
		"T_ARTWORK_BY": "Gràfics per"
	},
	"de": {
		"R_VERSION_START_ERROR": "Die Versionsanfrage konnte nicht gestartet werden (Engine-Status %d): %s",
		"R_VERSION_TRANSPORT_ERROR": "Die Versionsanfrage ist fehlgeschlagen (Transportstatus %d).",
		"R_VERSION_TIMEOUT": "Die Versionsanfrage hat das Zeitlimit von %d Sekunden überschritten.",
		"R_VERSION_BODY_LIMIT": "Die Versionsantwort überschreitet %d Bytes.",
		"R_VERSION_REDIRECT_LIMIT": "Die Versionsanfrage hat mehr als %d Weiterleitungen ausgelöst.",
		"R_VERSION_HTTP_ERROR": "Der Versionsserver hat HTTP %d zurückgegeben.",
		"R_VERSION_EMPTY": "Der Versionsserver hat eine leere Antwort zurückgegeben.",
		"R_VERSION_UTF8": "Der Versionsserver hat ungültigen UTF-8-Text zurückgegeben.",
		"T_CHECK_LAST_VERSION": "Auf die aktuellste Version überprüfen",
		"T_CHECK_LAST_VERSION_MNEMONIC": "V",
		"T_YOUR_VERSION": "Ihre Version:",
		"T_LAST_VERSION": "Neuste Version:",
		"T_VERSION_TITLE": "Version",
		"T_CANT_CHECK_CURRENT_VERSION": "Ich kann die aktuelle Version nicht überprüfen:",
		"T_HELP": "Hilfe",
		"T_USER_MANUAL": "Benutzerhandbuch",
		"T_ABOUT": "Über Biogenesis",
		"T_HELP_MNEMONIC": "H",
		"T_USER_MANUAL_MNEMONIC": "U",
		"T_ABOUT_MNEMONIC": "B",
		"T_PROGRAM_NAME": "Biogenesis\n",
		"T_VERSION_B": "Biogenesis Version:",
		"T_VERSION_CM": "Color Mod Version:",
		"T_VERSION_BB": "Backup Button Version:",
		"T_COPYRIGHT": "\nCopyright (c) Joan Queralt Molina  2006-2010\nColor Mod von MarcoDBAA 2012-2022\nBackup Button von Tyler Coleman 2013\nSTRtrees (von Vivid Solutions) implementiert von Richard David Williams 2017\n",
		"T_ARTWORK_BY": "Artwork by"
	},
	"pt": {
		"R_VERSION_START_ERROR": "Não foi possível iniciar a consulta de versão (código do motor %d): %s",
		"R_VERSION_TRANSPORT_ERROR": "A consulta de versão falhou (código de transporte %d).",
		"R_VERSION_TIMEOUT": "A consulta de versão excedeu o limite de %d segundos.",
		"R_VERSION_BODY_LIMIT": "A resposta de versão excede %d bytes.",
		"R_VERSION_REDIRECT_LIMIT": "A consulta de versão excedeu o limite de %d redirecionamentos.",
		"R_VERSION_HTTP_ERROR": "O servidor de versões devolveu HTTP %d.",
		"R_VERSION_EMPTY": "O servidor de versões devolveu uma resposta vazia.",
		"R_VERSION_UTF8": "O servidor de versões devolveu texto UTF-8 inválido.",
		"T_CHECK_LAST_VERSION": "Verificar a última versão",
		"T_CHECK_LAST_VERSION_MNEMONIC": "V",
		"T_YOUR_VERSION": "A sua versão:",
		"T_LAST_VERSION": "Última versão:",
		"T_VERSION_TITLE": "Versão",
		"T_CANT_CHECK_CURRENT_VERSION": "Eu não posso verificar a versão atual:",
		"T_HELP": "Ajuda",
		"T_USER_MANUAL": "Manual do Utilizador",
		"T_ABOUT": "Sobre Biogenesis",
		"T_HELP_MNEMONIC": "H",
		"T_USER_MANUAL_MNEMONIC": "M",
		"T_ABOUT_MNEMONIC": "A",
		"T_PROGRAM_NAME": "Biogenesis\n",
		"T_VERSION_B": "Versão do Biogenesis:",
		"T_VERSION_CM": "Versão de Color Mod:",
		"T_VERSION_BB": "Versão de Backup Button:",
		"T_COPYRIGHT": "\nDireitos de Autor (c) Joan Queralt Molina  2006-2010\nColor Mod de MarcoDBAA 2012-2022\nBackup Button de Tyler Coleman 2013\nSTRtrees (de Vivid Solutions) implementado por Richard David Williams 2017\n",
		"T_ARTWORK_BY": "Trabalho artístico"
	},
	"ru": {
		"R_VERSION_START_ERROR": "Не удалось запустить запрос версии (код движка %d): %s",
		"R_VERSION_TRANSPORT_ERROR": "Запрос версии завершился ошибкой (код транспорта %d).",
		"R_VERSION_TIMEOUT": "Запрос версии превысил время ожидания %d секунд.",
		"R_VERSION_BODY_LIMIT": "Ответ о версии превышает %d байт.",
		"R_VERSION_REDIRECT_LIMIT": "Запрос версии превысил предел в %d перенаправления.",
		"R_VERSION_HTTP_ERROR": "Сервер версий вернул HTTP %d.",
		"R_VERSION_EMPTY": "Сервер версий вернул пустой ответ.",
		"R_VERSION_UTF8": "Сервер версий вернул некорректный текст UTF-8.",
		"T_CHECK_LAST_VERSION": "Проверить последнюю версию",
		"T_CHECK_LAST_VERSION_MNEMONIC": "V",
		"T_YOUR_VERSION": "Ваша версия:",
		"T_LAST_VERSION": "Последняя версия:",
		"T_VERSION_TITLE": "Версия",
		"T_CANT_CHECK_CURRENT_VERSION": "Не могу проверить текущую версию:",
		"T_HELP": "Помощь",
		"T_USER_MANUAL": "Руководство пользователя",
		"T_ABOUT": "О программе Biogenesis",
		"T_HELP_MNEMONIC": "H",
		"T_USER_MANUAL_MNEMONIC": "M",
		"T_ABOUT_MNEMONIC": "A",
		"T_PROGRAM_NAME": "Biogenesis\n",
		"T_VERSION_B": "Biogenesis версия:",
		"T_VERSION_CM": "Color Mod версия:",
		"T_VERSION_BB": "Backup Button версия:",
		"T_COPYRIGHT": "\nCopyright (c) Joan Queralt Molina  2006-2010\nColor Mod от MarcoDBAA 2012-2022\nBackup Button от Tyler Coleman 2013\nSTRtrees (от Vivid Solutions) реализована Richard David Williams 2017\n",
		"T_ARTWORK_BY": "Дизайн"
	}
}

var language: String = "en"
var _closing: bool = false


static func message(key: String, locale: String) -> String:
	var messages: Dictionary = DATA.get(locale, DATA["en"])
	return str(messages.get(key, "!" + key + "!"))


static func manual_target(locale: String, manual_directory: String) -> String:
	for filename: String in ["manual_" + locale + ".html", "manual_en.html"]:
		var path: String = manual_directory.path_join(filename)
		if FileAccess.file_exists(path) or DirAccess.dir_exists_absolute(path):
			return path
	var remote_locale: String = locale if locale in ["ca", "es", "en"] else "en"
	return "http://biogenesis.sourceforge.net/manual." + remote_locale + ".php"


static func about_text(locale: String) -> String:
	return "Biogenesis-R\n\n" + message("T_PROGRAM_NAME", locale) + "http://biogenesis.sourceforge.net/\n" + message("T_VERSION_B", locale) + " 0.9\n" + message("T_VERSION_CM", locale) + " 2.01\n" + message("T_VERSION_BB", locale) + " 1\n" + message("T_COPYRIGHT", locale) + "joanq@users.sourceforge.net\n" + message("T_ARTWORK_BY", locale) + " Ananda Daydream, Florian Haag (http://toolbaricons.sourceforge.net/)"


static func open_manual(target: String, tree: SceneTree, report_error: Callable) -> int:
	if OS.get_name() not in ["Linux", "FreeBSD", "NetBSD", "OpenBSD"]:
		var opened: Error = OS.shell_open(target)
		if opened != OK:
			_report_open_error(report_error, error_string(opened))
		return -1
	var directory: DirAccess = DirAccess.open(".")
	if directory == null:
		_report_open_error(report_error, "Cannot resolve the current working directory")
		return -1
	var opener: String = _find_opener(OS.get_environment("PATH"), directory.get_current_dir())
	if opener.is_empty():
		_report_open_error(report_error, "Could not find web browser")
		return -1
	var arguments: PackedStringArray = PackedStringArray([target])
	if opener.get_file() == "gio":
		arguments.insert(0, "open")
	var pid: int = OS.create_process(opener, arguments)
	if pid < 0:
		_report_open_error(report_error, "Could not create browser process")
		return -1
	_reap_manual(pid, tree, report_error)
	return pid


static func _find_opener(search_path: String, working_directory: String) -> String:
	for name: String in OPENERS:
		for entry: String in search_path.split(":", true):
			var directory: String = entry if entry.is_absolute_path() else working_directory.path_join(entry)
			var candidate: String = directory.path_join(name)
			if FileAccess.file_exists(candidate) and (FileAccess.get_unix_permissions(candidate) & EXECUTE_BITS) != 0:
				return candidate
	return ""


static func _reap_manual(pid: int, tree: SceneTree, report_error: Callable) -> void:
	# SceneTree timers keep reaping independent of the launching Main window
	while is_instance_valid(tree):
		if not OS.is_process_running(pid):
			var status: int = OS.get_process_exit_code(pid)
			if status != 0:
				_report_open_error(report_error, "Browser process exited with status " + str(status))
			return
		await tree.create_timer(1.0, true, false, true).timeout


static func _report_open_error(report_error: Callable, detail: String) -> void:
	if report_error.is_valid():
		report_error.call("Error attempting to launch web browser:\n" + detail)


func _ready() -> void:
	title = message("T_ABOUT", language)
	force_native = true
	transient = true
	exclusive = true
	get_label().hide()
	confirmed.connect(close_about)
	canceled.connect(close_about)
	var content: HBoxContainer = HBoxContainer.new()
	content.add_theme_constant_override("separation", 12)
	var artwork: TextureRect = TextureRect.new()
	artwork.name = "AboutArtwork"
	artwork.texture = ARTWORK
	artwork.custom_minimum_size = Vector2(32, 32)
	artwork.size_flags_vertical = Control.SIZE_SHRINK_BEGIN
	artwork.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	content.add_child(artwork)
	var text: Label = Label.new()
	text.name = "AboutText"
	text.text = about_text(language)
	text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	text.custom_minimum_size.x = 500
	text.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	content.add_child(text)
	add_child(content)
	popup_centered_clamped(Vector2i(620, 400))
	get_ok_button().grab_focus()


func close_about() -> void:
	if _closing:
		return
	_closing = true
	hide()
	closed.emit()
	queue_free()
