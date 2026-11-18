# Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
# SPDX-License-Identifier: GPL-2.0-or-later

extends RefCounted

const DATA: Dictionary = {
	"en": {
		"T_NETWORK": "Network",
		"T_NETWORK_CONFIGURATION": "Network configuration",
		"T_MANAGE_CONNECTIONS": "Manage connections...",
		"T_NETWORK_CONNECTIONS": "Network connections",
		"T_NEW_CONNECTION": "New Connection",
		"T_ALLOW_CONNECTIONS_FROM_OTHER_USERS": "Allow connections from other users",
		"T_LOCAL_PORT_TO_RECEIVE_CONNECTIONS": "Local port to receive connections:",
		"T_MAXIMUM_NUMBER_OF_ALLOWED_CONNECTIONS": "Maximum number of allowed connections: ",
		"T_USER_NAME": "User name: ",
		"T_IP": "IP: ",
		"T_PORT": "Port: ",
		"T_DISCONNECT": "Disconnect",
		"T_OK": "OK",
		"T_CANCEL": "Cancel",
		"T_YES": "Yes",
		"T_NO": "No",
		"T_NETWORK_MNEMONIC": "N",
		"T_MANAGE_CONNECTIONS_MNEMONIC": "M",
		"R_LISTENING": "Listening on port %d",
		"R_STOPPED": "Listener stopped",
		"R_CONNECTIONS": "Connections: %d; incoming connection limit: %d",
		"R_QUEUES": "Incoming: %d; candidate: %s; outgoing: %d",
		"R_CONNECTING": "Connecting",
		"R_CONNECTED": "Connected",
		"R_DISCONNECTED": "Disconnected",
		"R_STATE": "State %d",
		"R_RESOLVING": "Resolving host...",
		"R_DNS_ERROR": "Cannot resolve the host.",
		"R_DNS_TIMEOUT": "Host lookup timed out.",
		"R_CANCEL_LOOKUP": "Cancel lookup",
		"R_INVALID_PORT": "Enter a remote port from 1 to 65535.",
		"R_LIMITS": "Limits: %d outgoing network requests at once and %d waiting arrivals per connection.",
		"R_EMPTY": "No connections.",
		"R_HOST_REQUIRED": "Enter a host or IP address.",
		"R_DNS_UNAVAILABLE": "Cannot start host lookup.",
		"R_BUSY": "Finish the open dialog before opening network controls.",
		"R_PRESENTATION_ERROR": "The simulation returned invalid network drawing data.",
		"T_CONFIGURE_NETWORK": "Configure network...",
		"T_CONFIGURE_NETWORK_MNEMONIC": "C"
	},
	"es": {
		"T_NETWORK": "Red",
		"T_NETWORK_CONFIGURATION": "Configurar la red",
		"T_MANAGE_CONNECTIONS": "Administrar conexiones...",
		"T_NETWORK_CONNECTIONS": "Conexiones de red",
		"T_NEW_CONNECTION": "Nueva conexión",
		"T_ALLOW_CONNECTIONS_FROM_OTHER_USERS": "Permitir conexiones de otros usuarios",
		"T_LOCAL_PORT_TO_RECEIVE_CONNECTIONS": "Puerto local para recibir conexiones:",
		"T_MAXIMUM_NUMBER_OF_ALLOWED_CONNECTIONS": "Número máximo de conexiones permitidas:",
		"T_USER_NAME": "Nombre de usuario:",
		"T_IP": "IP:",
		"T_PORT": "Puerto:",
		"T_DISCONNECT": "Desconectar",
		"T_OK": "Aceptar",
		"T_CANCEL": "Cancelar",
		"T_YES": "Sí",
		"T_NO": "No",
		"T_NETWORK_MNEMONIC": "R",
		"T_MANAGE_CONNECTIONS_MNEMONIC": "A",
		"R_LISTENING": "Escuchando en el puerto %d",
		"R_STOPPED": "Servidor detenido",
		"R_CONNECTIONS": "Conexiones: %d; límite de conexiones entrantes: %d",
		"R_QUEUES": "Entrantes: %d; candidato: %s; salientes: %d",
		"R_CONNECTING": "Conectando",
		"R_CONNECTED": "Conectado",
		"R_DISCONNECTED": "Desconectado",
		"R_STATE": "Estado %d",
		"R_RESOLVING": "Resolviendo el servidor...",
		"R_DNS_ERROR": "No se puede resolver el servidor.",
		"R_DNS_TIMEOUT": "La búsqueda del servidor agotó el tiempo.",
		"R_CANCEL_LOOKUP": "Cancelar búsqueda",
		"R_INVALID_PORT": "Introduzca un puerto remoto de 1 a 65535.",
		"R_LIMITS": "Límites: %d solicitudes de red salientes a la vez y %d llegadas en espera por conexión.",
		"R_EMPTY": "No hay conexiones.",
		"R_HOST_REQUIRED": "Introduzca un servidor o una direccion IP.",
		"R_DNS_UNAVAILABLE": "No se puede iniciar la búsqueda del servidor.",
		"R_BUSY": "Termine el diálogo abierto antes de abrir los controles de red.",
		"R_PRESENTATION_ERROR": "La simulacion devolvió datos de dibujo de red no válidos.",
		"T_CONFIGURE_NETWORK": "Configurar la red...",
		"T_CONFIGURE_NETWORK_MNEMONIC": "C"
	},
	"ca": {
		"T_NETWORK": "Xarxa",
		"T_NETWORK_CONFIGURATION": "Configura la xarxa",
		"T_MANAGE_CONNECTIONS": "Administra les connexions...",
		"T_NETWORK_CONNECTIONS": "Connexions de xarxa",
		"T_NEW_CONNECTION": "Nova connexió",
		"T_ALLOW_CONNECTIONS_FROM_OTHER_USERS": "Permet connexions d'altres usuaris",
		"T_LOCAL_PORT_TO_RECEIVE_CONNECTIONS": "Port local per rebre connexions:",
		"T_MAXIMUM_NUMBER_OF_ALLOWED_CONNECTIONS": "Nombre màxim de connexions permeses:",
		"T_USER_NAME": "Nom d’usuari:",
		"T_IP": "IP:",
		"T_PORT": "Port:",
		"T_DISCONNECT": "Desconnecta",
		"T_OK": "Accepta",
		"T_CANCEL": "Cancel·la",
		"T_YES": "Sí",
		"T_NO": "No",
		"T_NETWORK_MNEMONIC": "X",
		"T_MANAGE_CONNECTIONS_MNEMONIC": "A",
		"R_LISTENING": "Escoltant al port %d",
		"R_STOPPED": "Servidor aturat",
		"R_CONNECTIONS": "Connexions: %d; límit de connexions entrants: %d",
		"R_QUEUES": "Entrants: %d; candidat: %s; sortints: %d",
		"R_CONNECTING": "Connectant",
		"R_CONNECTED": "Connectat",
		"R_DISCONNECTED": "Desconnectat",
		"R_STATE": "Estat %d",
		"R_RESOLVING": "Resolent el servidor...",
		"R_DNS_ERROR": "No es pot resoldre el servidor.",
		"R_DNS_TIMEOUT": "La cerca del servidor ha esgotat el temps.",
		"R_CANCEL_LOOKUP": "Cancel·la la cerca",
		"R_INVALID_PORT": "Introduiu un port remot de l'1 al 65535.",
		"R_LIMITS": "Límits: %d peticions de xarxa sortints alhora i %d arribades en espera per connexió.",
		"R_EMPTY": "No hi ha connexions.",
		"R_HOST_REQUIRED": "Introduiu un servidor o una adreça IP.",
		"R_DNS_UNAVAILABLE": "No es pot iniciar la cerca del servidor.",
		"R_BUSY": "Acabeu el diàleg obert abans d'obrir els controls de xarxa.",
		"R_PRESENTATION_ERROR": "La simulació ha retornat dades de dibuix de xarxa no vàlides.",
		"T_CONFIGURE_NETWORK": "Configura la xarxa...",
		"T_CONFIGURE_NETWORK_MNEMONIC": "C"
	},
	"de": {
		"T_NETWORK": "Netzwerk",
		"T_NETWORK_CONFIGURATION": "Netzwerkkonfiguration",
		"T_MANAGE_CONNECTIONS": "Verbindungen verwalten...",
		"T_NETWORK_CONNECTIONS": "Netzwerkverbindungen",
		"T_NEW_CONNECTION": "Neue Verbindung",
		"T_ALLOW_CONNECTIONS_FROM_OTHER_USERS": "Verbindungen mit anderen Benutzern erlauben",
		"T_LOCAL_PORT_TO_RECEIVE_CONNECTIONS": "Lokaler Port um Verbindungen zu erhalten:",
		"T_MAXIMUM_NUMBER_OF_ALLOWED_CONNECTIONS": "Maximale Anzahl an erlaubten Verbindungen:",
		"T_USER_NAME": "Benutzername:",
		"T_IP": "IP:",
		"T_PORT": "Port:",
		"T_DISCONNECT": "Trennen",
		"T_OK": "OK",
		"T_CANCEL": "Abbrechen",
		"T_YES": "Ja",
		"T_NO": "Nein",
		"T_NETWORK_MNEMONIC": "N",
		"T_MANAGE_CONNECTIONS_MNEMONIC": "V",
		"R_LISTENING": "Empfang auf Port %d",
		"R_STOPPED": "Server angehalten",
		"R_CONNECTIONS": "Verbindungen: %d; Limit für eingehende Verbindungen: %d",
		"R_QUEUES": "Eingehend: %d; Kandidat: %s; ausgehend: %d",
		"R_CONNECTING": "Verbindung wird aufgebaut",
		"R_CONNECTED": "Verbunden",
		"R_DISCONNECTED": "Getrennt",
		"R_STATE": "Zustand %d",
		"R_RESOLVING": "Host wird aufgelöst...",
		"R_DNS_ERROR": "Host kann nicht aufgelöst werden.",
		"R_DNS_TIMEOUT": "Zeitlimit bei der Hostsuche erreicht.",
		"R_CANCEL_LOOKUP": "Suche abbrechen",
		"R_INVALID_PORT": "Geben Sie einen entfernten Port von 1 bis 65535 ein.",
		"R_LIMITS": "Grenzen: %d gleichzeitige ausgehende Netzwerkanfragen und %d wartende Ankünfte je Verbindung.",
		"R_EMPTY": "Keine Verbindungen.",
		"R_HOST_REQUIRED": "Geben Sie einen Host oder eine IP-Adresse ein.",
		"R_DNS_UNAVAILABLE": "Hostsuche kann nicht gestartet werden.",
		"R_BUSY": "Schließen Sie den offenen Dialog, bevor Sie die Netzwerksteuerung öffnen.",
		"R_PRESENTATION_ERROR": "Die Simulation hat ungültige Netzwerk-Zeichendaten geliefert.",
		"T_CONFIGURE_NETWORK": "Netzwerk konfigurieren...",
		"T_CONFIGURE_NETWORK_MNEMONIC": "K"
	},
	"pt": {
		"T_NETWORK": "Rede",
		"T_NETWORK_CONFIGURATION": "Configuração da rede",
		"T_MANAGE_CONNECTIONS": "Gerir ligações...",
		"T_NETWORK_CONNECTIONS": "Conexões de rede",
		"T_NEW_CONNECTION": "Nova conexão",
		"T_ALLOW_CONNECTIONS_FROM_OTHER_USERS": "Aceitar conexões de outros utilizadores",
		"T_LOCAL_PORT_TO_RECEIVE_CONNECTIONS": "Porta local para receber conexões:",
		"T_MAXIMUM_NUMBER_OF_ALLOWED_CONNECTIONS": "Número máximo de conexões aceites:",
		"T_USER_NAME": "Nome do usuário:",
		"T_IP": "IP:",
		"T_PORT": "Porta:",
		"T_DISCONNECT": "Desconectar",
		"T_OK": "OK",
		"T_CANCEL": "Cancelar",
		"T_YES": "Sim",
		"T_NO": "Não",
		"T_NETWORK_MNEMONIC": "N",
		"T_MANAGE_CONNECTIONS_MNEMONIC": "M",
		"R_LISTENING": "A escutar na porta %d",
		"R_STOPPED": "Servidor parado",
		"R_CONNECTIONS": "Ligações: %d; limite de ligações de entrada: %d",
		"R_QUEUES": "Entrada: %d; candidato: %s; saída: %d",
		"R_CONNECTING": "A ligar",
		"R_CONNECTED": "Ligado",
		"R_DISCONNECTED": "Desligado",
		"R_STATE": "Estado %d",
		"R_RESOLVING": "A resolver o servidor...",
		"R_DNS_ERROR": "Não foi possivel resolver o servidor.",
		"R_DNS_TIMEOUT": "A pesquisa do servidor excedeu o tempo limite.",
		"R_CANCEL_LOOKUP": "Cancelar pesquisa",
		"R_INVALID_PORT": "Introduza uma porta remota de 1 a 65535.",
		"R_LIMITS": "Limites: %d pedidos de rede de saída em simultâneo e %d chegadas em espera por ligação.",
		"R_EMPTY": "Sem ligações.",
		"R_HOST_REQUIRED": "Introduza um servidor ou um endereço IP.",
		"R_DNS_UNAVAILABLE": "Não foi possivel iniciar a pesquisa do servidor.",
		"R_BUSY": "Termine o diálogo aberto antes de abrir os controlos de rede.",
		"R_PRESENTATION_ERROR": "A simulação devolveu dados de desenho de rede inválidos.",
		"T_CONFIGURE_NETWORK": "Configurar rede...",
		"T_CONFIGURE_NETWORK_MNEMONIC": "C"
	},
	"ru": {
		"T_NETWORK": "Сеть",
		"T_NETWORK_CONFIGURATION": "Конфигурация сети",
		"T_MANAGE_CONNECTIONS": "Управление соединениями...",
		"T_NETWORK_CONNECTIONS": "Сетевые соединения",
		"T_NEW_CONNECTION": "Новое соединение",
		"T_ALLOW_CONNECTIONS_FROM_OTHER_USERS": "Позволить соединения от других пользователей",
		"T_LOCAL_PORT_TO_RECEIVE_CONNECTIONS": "Локальный порт для получения соединений:",
		"T_MAXIMUM_NUMBER_OF_ALLOWED_CONNECTIONS": "Максимальное число позволенных соединений:",
		"T_USER_NAME": "Имя пользователя:",
		"T_IP": "IP:",
		"T_PORT": "Порт:",
		"T_DISCONNECT": "Разъединить",
		"T_OK": "ОК",
		"T_CANCEL": "Отмена",
		"T_YES": "Да",
		"T_NO": "Нет",
		"T_NETWORK_MNEMONIC": "N",
		"T_MANAGE_CONNECTIONS_MNEMONIC": "M",
		"R_LISTENING": "Прием на порту %d",
		"R_STOPPED": "Сервер остановлен",
		"R_CONNECTIONS": "Соединений: %d; предел входящих соединений: %d",
		"R_QUEUES": "Входящих: %d; кандидат: %s; исходящих: %d",
		"R_CONNECTING": "Подключение",
		"R_CONNECTED": "Подключено",
		"R_DISCONNECTED": "Отключено",
		"R_STATE": "Состояние %d",
		"R_RESOLVING": "Поиск адреса...",
		"R_DNS_ERROR": "Не удалось определить адрес.",
		"R_DNS_TIMEOUT": "Истекло время поиска адреса.",
		"R_CANCEL_LOOKUP": "Отменить поиск",
		"R_INVALID_PORT": "Введите удаленный порт от 1 до 65535.",
		"R_LIMITS": "Пределы: %d исходящих сетевых запросов одновременно и %d ожидающих поступлений на соединение.",
		"R_EMPTY": "Нет соединений.",
		"R_HOST_REQUIRED": "Введите имя узла или IP-адрес.",
		"R_DNS_UNAVAILABLE": "Не удалось начать поиск адреса.",
		"R_BUSY": "Закройте открытый диалог перед открытием сетевого управления.",
		"R_PRESENTATION_ERROR": "Симуляция вернула неверные данные рисования сети.",
		"T_CONFIGURE_NETWORK": "Настройка сети...",
		"T_CONFIGURE_NETWORK_MNEMONIC": "C"
	}
}


static func message(key: String, language: String) -> String:
	var values: Dictionary = DATA.get(language, DATA["en"])
	return str(values.get(key, "!" + key + "!"))


static func mnemonic(key: String, language: String) -> int:
	var values: Dictionary = DATA.get(language, DATA["en"])
	return OS.find_keycode_from_string(str(values.get(key + "_MNEMONIC", message(key, language).left(1))).to_upper())


const FEEDBACK: Dictionary = {
	"en": {
		"listening": "Listening on port %d; at most %d incoming genomes per peer.",
		"connecting": "Connecting to %s.",
		"connected": "Connected with %s.",
		"closed": "Connection closed with %s.",
		"closing": "Closing network connections.",
		"all_closed": "Network connections closed.",
		"cancelled": "Remaining network notifications were cancelled.",
		"invalid_local_port": "Enter a local port from 0 to 65535. Zero selects an available port.",
		"invalid_remote_port": "Enter a remote port from 1 to 65535.",
		"invalid_address": "Resolve the host to an IP address before connecting.",
		"invalid_identity": "Select a valid connection.",
		"already_closed": "This connection has already closed.",
		"world_required": "Create a world before connecting.",
		"closing_previous": "Previous connections are still closing. Wait for cleanup, then retry.",
		"duplicate": "This address and port already has a connection.",
		"capacity": "Too many outgoing network requests are in progress. Wait, then retry.",
		"worker_capacity": "Too many outgoing network requests are in progress. Wait, then retry.",
		"refused": "The peer refused the connection (protocol reply %d). Check its settings and version.",
		"listener_failed": "Cannot start or maintain the listener. Check the local port and system diagnostics.",
		"transfer_failed": "The transfer did not complete. Check the connection and diagnostics.",
		"incoming_failed": "The incoming transfer was not acknowledged. Check the connection and diagnostics.",
		"preferences_failed": "Network settings changed, but preferences could not be saved. The listener port was retained.",
		"cleanup_failed": "Network cleanup failed. Check the diagnostics before reconnecting.",
		"operation_failed": "The network operation failed. Check the address, connection and diagnostics.",
		"invalid_feedback": "The network returned invalid feedback data.",
		"diagnostic_detail": "Technical details: %s"
	},
	"es": {
		"listening": "Escuchando en el puerto %d; máximo %d genomas entrantes por conexión.",
		"connecting": "Conectando con %s.",
		"connected": "Conectado con %s.",
		"closed": "Conexión cerrada con %s.",
		"closing": "Cerrando las conexiones de red.",
		"all_closed": "Conexiones de red cerradas.",
		"cancelled": "Se cancelaron las notificaciones de red restantes.",
		"invalid_local_port": "Introduzca un puerto local de 0 a 65535. Cero elige un puerto disponible.",
		"invalid_remote_port": "Introduzca un puerto remoto de 1 a 65535.",
		"invalid_address": "Resuelva el nombre a una dirección IP antes de conectar.",
		"invalid_identity": "Seleccione una conexión válida.",
		"already_closed": "Esta conexión ya se ha cerrado.",
		"world_required": "Cree un mundo antes de conectar.",
		"closing_previous": "Las conexiones anteriores siguen cerrándose. Espere a que termine la limpieza y reintente.",
		"duplicate": "Ya existe una conexión con esta dirección y puerto.",
		"capacity": "Hay demasiadas solicitudes de red salientes en curso. Espere y vuelva a intentarlo.",
		"worker_capacity": "Hay demasiadas solicitudes de red salientes en curso. Espere y vuelva a intentarlo.",
		"refused": "El otro equipo rechazó la conexión (respuesta del protocolo %d). Revise su configuración y versión.",
		"listener_failed": "No se puede iniciar o mantener la escucha. Revise el puerto local y el diagnóstico del sistema.",
		"transfer_failed": "La transferencia no se completó. Revise la conexión y el diagnóstico.",
		"incoming_failed": "No se confirmó la transferencia entrante. Revise la conexión y el diagnóstico.",
		"preferences_failed": "La configuración de red cambió, pero no se pudieron guardar las preferencias. Se conservó el puerto de escucha.",
		"cleanup_failed": "Falló la limpieza de red. Revise el diagnóstico antes de volver a conectar.",
		"operation_failed": "Falló la operación de red. Revise la dirección, la conexión y el diagnóstico.",
		"invalid_feedback": "La red devolvió datos de aviso no válidos.",
		"diagnostic_detail": "Detalles técnicos: %s"
	},
	"ca": {
		"listening": "Escoltant al port %d; màxim %d genomes entrants per connexió.",
		"connecting": "Connectant amb %s.",
		"connected": "Connectat amb %s.",
		"closed": "Connexió tancada amb %s.",
		"closing": "Tancant les connexions de xarxa.",
		"all_closed": "Connexions de xarxa tancades.",
		"cancelled": "S'han cancel·lat les notificacions de xarxa restants.",
		"invalid_local_port": "Introduïu un port local de 0 a 65535. Zero tria un port disponible.",
		"invalid_remote_port": "Introduïu un port remot d’1 a 65535.",
		"invalid_address": "Resoleu el nom a una adreça IP abans de connectar.",
		"invalid_identity": "Seleccioneu una connexió vàlida.",
		"already_closed": "Aquesta connexió ja està tancada.",
		"world_required": "Creeu un món abans de connectar.",
		"closing_previous": "Les connexions anteriors encara es tanquen. Espereu que acabi la neteja i torneu-ho a provar.",
		"duplicate": "Ja hi ha una connexió amb aquesta adreça i port.",
		"capacity": "Hi ha massa peticions de xarxa sortints en curs. Espereu i torneu-ho a provar.",
		"worker_capacity": "Hi ha massa peticions de xarxa sortints en curs. Espereu i torneu-ho a provar.",
		"refused": "L’altre equip ha rebutjat la connexió (resposta del protocol %d). Reviseu-ne la configuració i la versió.",
		"listener_failed": "No es pot iniciar o mantenir l’escolta. Reviseu el port local i el diagnòstic del sistema.",
		"transfer_failed": "La transferència no s’ha completat. Reviseu la connexió i el diagnòstic.",
		"incoming_failed": "No s’ha confirmat la transferència entrant. Reviseu la connexió i el diagnòstic.",
		"preferences_failed": "La configuració de xarxa ha canviat, però no s’han pogut desar les preferències. S’ha conservat el port d’escolta.",
		"cleanup_failed": "La neteja de xarxa ha fallat. Reviseu el diagnòstic abans de tornar a connectar.",
		"operation_failed": "L’operació de xarxa ha fallat. Reviseu l’adreça, la connexió i el diagnòstic.",
		"invalid_feedback": "La xarxa ha retornat dades d’avís no vàlides.",
		"diagnostic_detail": "Detalls tècnics: %s"
	},
	"de": {
		"listening": "Empfang auf Port %d; höchstens %d eingehende Genome pro Verbindung.",
		"connecting": "Verbindung mit %s wird aufgebaut.",
		"connected": "Mit %s verbunden.",
		"closed": "Verbindung mit %s geschlossen.",
		"closing": "Netzwerkverbindungen werden geschlossen.",
		"all_closed": "Netzwerkverbindungen geschlossen.",
		"cancelled": "Verbleibende Netzwerkbenachrichtigungen wurden abgebrochen.",
		"invalid_local_port": "Geben Sie einen lokalen Port von 0 bis 65535 ein. Null wählt einen freien Port.",
		"invalid_remote_port": "Geben Sie einen entfernten Port von 1 bis 65535 ein.",
		"invalid_address": "Lösen Sie den Rechnernamen vor dem Verbinden in eine IP-Adresse auf.",
		"invalid_identity": "Wählen Sie eine gültige Verbindung.",
		"already_closed": "Diese Verbindung ist bereits geschlossen.",
		"world_required": "Erstellen Sie vor dem Verbinden eine Welt.",
		"closing_previous": "Vorherige Verbindungen werden noch geschlossen. Warten Sie auf die Bereinigung und versuchen Sie es erneut.",
		"duplicate": "Für diese Adresse und diesen Port besteht bereits eine Verbindung.",
		"capacity": "Zu viele ausgehende Netzwerkanfragen laufen. Warten Sie und versuchen Sie es erneut.",
		"worker_capacity": "Zu viele ausgehende Netzwerkanfragen laufen. Warten Sie und versuchen Sie es erneut.",
		"refused": "Die Gegenstelle hat die Verbindung abgelehnt (Protokollantwort %d). Prüfen Sie deren Einstellungen und Version.",
		"listener_failed": "Der Empfang kann nicht gestartet oder fortgesetzt werden. Prüfen Sie den lokalen Port und die Systemdiagnose.",
		"transfer_failed": "Die Übertragung wurde nicht abgeschlossen. Prüfen Sie die Verbindung und die Diagnose.",
		"incoming_failed": "Die eingehende Übertragung wurde nicht bestätigt. Prüfen Sie die Verbindung und die Diagnose.",
		"preferences_failed": "Die Netzwerkeinstellungen wurden geändert, aber nicht gespeichert. Der Empfangsport wurde beibehalten.",
		"cleanup_failed": "Die Netzwerkbereinigung ist fehlgeschlagen. Prüfen Sie vor dem erneuten Verbinden die Diagnose.",
		"operation_failed": "Der Netzwerkvorgang ist fehlgeschlagen. Prüfen Sie Adresse, Verbindung und Diagnose.",
		"invalid_feedback": "Das Netzwerk hat ungültige Meldungsdaten geliefert.",
		"diagnostic_detail": "Technische Details: %s"
	},
	"pt": {
		"listening": "A escutar na porta %d; máximo de %d genomas recebidos por ligação.",
		"connecting": "A ligar a %s.",
		"connected": "Ligado a %s.",
		"closed": "Ligação fechada com %s.",
		"closing": "A fechar as ligações de rede.",
		"all_closed": "Ligações de rede fechadas.",
		"cancelled": "As notificações de rede restantes foram canceladas.",
		"invalid_local_port": "Introduza uma porta local de 0 a 65535. Zero escolhe uma porta disponível.",
		"invalid_remote_port": "Introduza uma porta remota de 1 a 65535.",
		"invalid_address": "Resolva o nome para um endereço IP antes de ligar.",
		"invalid_identity": "Selecione uma ligação válida.",
		"already_closed": "Esta ligação já foi fechada.",
		"world_required": "Crie um mundo antes de ligar.",
		"closing_previous": "As ligações anteriores ainda estão a fechar. Aguarde a limpeza e tente novamente.",
		"duplicate": "Já existe uma ligação com este endereço e porta.",
		"capacity": "Há demasiados pedidos de rede de saída em curso. Aguarde e tente novamente.",
		"worker_capacity": "Há demasiados pedidos de rede de saída em curso. Aguarde e tente novamente.",
		"refused": "O outro computador recusou a ligação (resposta do protocolo %d). Verifique a configuração e a versão.",
		"listener_failed": "Não é possível iniciar ou manter a escuta. Verifique a porta local e o diagnóstico do sistema.",
		"transfer_failed": "A transferência não foi concluída. Verifique a ligação e o diagnóstico.",
		"incoming_failed": "A transferência recebida não foi confirmada. Verifique a ligação e o diagnóstico.",
		"preferences_failed": "A configuração de rede mudou, mas não foi possível guardar as preferências. A porta de escuta foi mantida.",
		"cleanup_failed": "A limpeza da rede falhou. Verifique o diagnóstico antes de voltar a ligar.",
		"operation_failed": "A operação de rede falhou. Verifique o endereço, a ligação e o diagnóstico.",
		"invalid_feedback": "A rede devolveu dados de aviso inválidos.",
		"diagnostic_detail": "Detalhes técnicos: %s"
	},
	"ru": {
		"listening": "Ожидание на порту %d; не более %d входящих геномов на соединение.",
		"connecting": "Подключение к %s.",
		"connected": "Соединение с %s установлено.",
		"closed": "Соединение с %s закрыто.",
		"closing": "Закрытие сетевых соединений.",
		"all_closed": "Сетевые соединения закрыты.",
		"cancelled": "Оставшиеся сетевые уведомления отменены.",
		"invalid_local_port": "Введите локальный порт от 0 до 65535. Ноль выбирает свободный порт.",
		"invalid_remote_port": "Введите удаленный порт от 1 до 65535.",
		"invalid_address": "Перед подключением определите IP-адрес узла.",
		"invalid_identity": "Выберите допустимое соединение.",
		"already_closed": "Это соединение уже закрыто.",
		"world_required": "Создайте мир перед подключением.",
		"closing_previous": "Предыдущие соединения еще закрываются. Дождитесь очистки и повторите попытку.",
		"duplicate": "Для этого адреса и порта уже есть соединение.",
		"capacity": "Слишком много исходящих сетевых запросов выполняется. Подождите и повторите попытку.",
		"worker_capacity": "Слишком много исходящих сетевых запросов выполняется. Подождите и повторите попытку.",
		"refused": "Узел отклонил соединение (ответ протокола %d). Проверьте его настройки и версию.",
		"listener_failed": "Не удалось начать или продолжить прием соединений. Проверьте локальный порт и диагностику системы.",
		"transfer_failed": "Передача не завершена. Проверьте соединение и диагностику.",
		"incoming_failed": "Входящая передача не подтверждена. Проверьте соединение и диагностику.",
		"preferences_failed": "Настройки сети изменены, но сохранить параметры не удалось. Порт приема сохранен.",
		"cleanup_failed": "Ошибка очистки сети. Перед повторным подключением проверьте диагностику.",
		"operation_failed": "Сетевая операция не выполнена. Проверьте адрес, соединение и диагностику.",
		"invalid_feedback": "Сеть вернула неверные данные сообщения.",
		"diagnostic_detail": "Технические сведения: %s"
	},
}

const FEEDBACK_ARGUMENTS: Dictionary = {
	"listening": ["port", "limit"],
	"connecting": ["address"],
	"connected": ["address"],
	"closed": ["address"],
	"closing": [],
	"all_closed": [],
	"cancelled": [],
	"invalid_local_port": [],
	"invalid_remote_port": [],
	"invalid_address": [],
	"invalid_identity": [],
	"already_closed": [],
	"world_required": [],
	"closing_previous": [],
	"duplicate": [],
	"capacity": [],
	"worker_capacity": [],
	"refused": ["reply"],
	"listener_failed": [],
	"transfer_failed": [],
	"incoming_failed": [],
	"preferences_failed": [],
	"cleanup_failed": [],
	"operation_failed": []
}

const NOTICE_CODES: Array[String] = ["listening", "connecting", "connected", "closed", "closing", "all_closed", "cancelled"]
const DETAIL_CODES: Array[String] = ["listener_failed", "transfer_failed", "incoming_failed", "preferences_failed", "cleanup_failed", "operation_failed"]


static func _invalid_feedback(raw: String, language: String) -> String:
	var values: Dictionary = FEEDBACK.get(language, FEEDBACK["en"])
	var text: String = values["invalid_feedback"]
	return text if raw.is_empty() else text + "\n" + str(values["diagnostic_detail"]) % raw


static func feedback(packet: Dictionary, channel: String, language: String) -> String:
	if channel != "notice" and channel != "error":
		return _invalid_feedback("", language)
	var raw: String = str(packet.get(channel, ""))
	if not packet.get(channel) is String:
		return _invalid_feedback(raw, language)
	var value: Variant = packet.get(channel + "_feedback")
	if not value is Dictionary:
		return _invalid_feedback(raw, language)
	var descriptor: Dictionary = value
	if descriptor.is_empty():
		return "" if raw.is_empty() else _invalid_feedback(raw, language)
	if raw.is_empty() or descriptor.size() != 3 or not descriptor.get("code") is String or not descriptor.get("arguments") is Dictionary or not descriptor.get("detail") is String:
		return _invalid_feedback(raw, language)
	var code: String = descriptor["code"]
	if not FEEDBACK_ARGUMENTS.has(code) or (code in NOTICE_CODES) != (channel == "notice"):
		return _invalid_feedback(raw, language)
	if not str(descriptor["detail"]).is_empty() and code not in DETAIL_CODES:
		return _invalid_feedback(raw, language)
	var arguments: Dictionary = descriptor["arguments"]
	var names: Array = FEEDBACK_ARGUMENTS[code]
	if arguments.size() != names.size():
		return _invalid_feedback(raw, language)
	var parameters: Array = []
	for name: String in names:
		if not arguments.has(name):
			return _invalid_feedback(raw, language)
		var argument: Variant = arguments[name]
		if name == "address":
			if not argument is String or str(argument).is_empty():
				return _invalid_feedback(raw, language)
		else:
			if not argument is int:
				return _invalid_feedback(raw, language)
			if name == "port" and (int(argument) < 0 or int(argument) > 65535):
				return _invalid_feedback(raw, language)
			if name == "limit" and int(argument) <= 0:
				return _invalid_feedback(raw, language)
			if name == "reply" and (int(argument) < -2147483648 or int(argument) > 2147483647):
				return _invalid_feedback(raw, language)
		parameters.append(argument)
	var values: Dictionary = FEEDBACK.get(language, FEEDBACK["en"])
	var text: String = values[code]
	if not parameters.is_empty():
		text = text % parameters
	var detail: String = descriptor["detail"]
	if not detail.is_empty():
		text += "\n" + str(values["diagnostic_detail"]) % detail
	return text
