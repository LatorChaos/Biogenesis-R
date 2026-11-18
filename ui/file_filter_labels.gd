# Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
# SPDX-License-Identifier: GPL-2.0-or-later

extends RefCounted

const DATA: Dictionary = {
	"en": {
		"T_BIOGENESIS_WORLD_FILES": "Biogenesis world files (*.bgw)",
		"T_BIOGENESIS_GENETIC_CODE_FILES": "Biogenesis genetic code files (*.bgg)",
		"T_PNG_IMAGE_FILES": "PNG image files (*.png)",
		"T_ALL_FILES": "All files"
	},
	"es": {
		"T_BIOGENESIS_WORLD_FILES": "Ficheros de mundo de Biogénesis (*.bgw)",
		"T_BIOGENESIS_GENETIC_CODE_FILES": "Ficheros de código genético de Biogénesis (*.bgg)",
		"T_PNG_IMAGE_FILES": "Ficheros de imágenes PNG (*.png)",
		"T_ALL_FILES": "Todos los archivos"
	},
	"ca": {
		"T_BIOGENESIS_WORLD_FILES": "Fitxers de món de Biogènesi (*.bgw)",
		"T_BIOGENESIS_GENETIC_CODE_FILES": "Fitxers de codi genètic de Biogènesi (*.bgg)",
		"T_PNG_IMAGE_FILES": "Fitxers d'imatges PNG (*.png)",
		"T_ALL_FILES": "Tots els fitxers"
	},
	"de": {
		"T_BIOGENESIS_WORLD_FILES": "Biogenesis world files (*.bgw)",
		"T_BIOGENESIS_GENETIC_CODE_FILES": "Biogenesis genetic code files (*.bgg)",
		"T_PNG_IMAGE_FILES": "PNG image files (*.png)",
		"T_ALL_FILES": "Alle Dateien"
	},
	"pt": {
		"T_BIOGENESIS_WORLD_FILES": "Ficheiros do ecossistema do Biogenesis (*.bgw)",
		"T_BIOGENESIS_GENETIC_CODE_FILES": "Ficheiros de código genético do Biogenesis (*.bgg)",
		"T_PNG_IMAGE_FILES": "Ficheiros de imagem PNG (*.png)",
		"T_ALL_FILES": "Todos os ficheiros"
	},
	"ru": {
		"T_BIOGENESIS_WORLD_FILES": "Файлы миров Biogenesis (*.bgw)",
		"T_BIOGENESIS_GENETIC_CODE_FILES": "Файлы генетических кодов Biogenesis (*.bgg)",
		"T_PNG_IMAGE_FILES": "Файлы изображений PNG (*.png)",
		"T_ALL_FILES": "Все файлы"
	}
}


static func message(key: String, language: String) -> String:
	var messages: Dictionary = DATA.get(language, DATA["en"])
	return str(messages.get(key, "!" + key + "!"))


static func filter_entry(pattern: String, key: String, language: String) -> String:
	# Godot appends the pattern to custom and native filter descriptions
	var description: String = message(key, language).trim_suffix(" (" + pattern + ")")
	return pattern + " ; " + description


static func fit_filter_labels(dialog: FileDialog) -> void:
	for child: Node in dialog.get_line_edit().get_parent().get_children():
		if child is OptionButton:
			var filter: OptionButton = child as OptionButton
			filter.clip_text = false
			filter.fit_to_longest_item = true
