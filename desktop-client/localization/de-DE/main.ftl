# Main Menu
menu-enter-world = Die Welt betreten
menu-quit-application = Anwendung beenden
menu-select-world = Eine Welt auswählen
menu-create-new-world = Neue Welt erstellen
menu-return-to-game = Zurück zum Spiel
menu-save-and-quit = Speichern und zum Hauptmenü
menu-quit-no-save = Zum Hauptmenü (ohne Speichern)
menu-language = Sprache:
menu-settings = Einstellungen
menu-back-to-main = Zurück zum Hauptmenü

# World Selection
world-last-played = Zuletzt gespielt: {$time}
world-unknown-time = Unbekannt

# Inventory and Items
item-grass = Gras
item-dirt = Erde
item-stone = Stein
item-quartz-block = Quarzblock
item-glass-pane = Glasscheibe
item-cyan-terracotta = Türkise Terrakotta
inventory-empty = Leer

# Console Commands
console-blink-started = Blinken gestartet
console-blink-stopped = Blinken gestoppt
console-blink-usage = Verwendung: blink [start|stop]
console-mqtt-connected = Mit MQTT-Broker verbunden
console-mqtt-connecting = Verbinde mit MQTT-Broker...
console-mqtt-temperature = Aktuelle Temperatur: {$temp}°C
console-mqtt-no-temperature = Keine Temperaturdaten verfügbar
console-mqtt-usage = Verwendung: mqtt [status|temp]
console-placed-block = {$block_type}-Block bei ({$x}, {$y}, {$z}) platziert
console-removed-block = Block bei ({$x}, {$y}, {$z}) entfernt
console-no-block-found = Kein Block bei ({$x}, {$y}, {$z}) gefunden
console-teleported = Teleportiert zu ({$x}, {$y}, {$z})
console-look-set = Blickwinkel gesetzt auf Yaw: {$yaw}°, Pitch: {$pitch}°
console-map-saved = Karte in '{$filename}' mit {$count} Blöcken gespeichert
console-map-loaded = Karte aus '{$filename}' mit {$count} Blöcken geladen
console-map-save-failed = Fehler beim Speichern der Karte: {$error}
console-map-load-failed = Fehler beim Laden der Karte: {$error}
console-script-loaded = {$count} Befehle aus {$filename} geladen
console-script-load-failed = Fehler beim Laden des Skripts {$filename}: {$error}
console-spawn-sent = Spawn-Befehl für Gerät {$device_id} gesendet
console-wall-created = Wand aus {$block_type} von ({$x1}, {$y1}, {$z1}) bis ({$x2}, {$y2}, {$z2}) erstellt
console-gave-items = {$quantity} x {$item_type} hinzugefügt
console-invalid-block-type = Ungültiger Blocktyp: {$block_type}
console-invalid-item-type = Ungültiger Gegenstandstyp: {$item_type}
console-unknown-command = Unbekannter Befehl: {$command}

# Diagnostics
debug-title = IoTCraft Debug-Informationen (F3 drücken zum Umschalten)
debug-divider = ------------------------------------------------------------------------------------------
debug-player-info = - SPIELER-INFORMATIONEN
debug-position = Position: X={$x}  Y={$y}  Z={$z}
debug-rotation = Rotation: Yaw={$yaw}°  Pitch={$pitch}°
debug-selected-slot = Ausgewählter Slot: {$slot} ({$item})
debug-world-info = - WELT-INFORMATIONEN
debug-total-blocks = Gesamte Blöcke: {$count}
debug-iot-devices = IoT-Geräte: {$count}
debug-session-time = Sitzungszeit: {$minutes}m {$seconds}s
debug-script-commands = - SKRIPT-BEFEHLE
debug-teleport = Teleportieren: tp {$x} {$y} {$z}
debug-look-direction = Blickrichtung: look {$yaw} {$pitch}
debug-controls = - STEUERUNG
debug-f3-toggle = F3: Diese Debug-Anzeige umschalten
debug-console-open = T: Konsole öffnen
debug-inventory-select = 1-9: Inventar-Slot auswählen
debug-inventory-scroll = Mausrad: Inventar-Slots scrollen

# Error Messages
error-camera-not-found = Fehler: Kamera nicht gefunden
error-camera-teleport-failed = Fehler: Kamera zum Teleportieren nicht gefunden
error-camera-look-failed = Fehler: Kamera zum Setzen der Blickrichtung nicht gefunden

# Device Messages
device-announce = Gerät {$device_id} angekündigt
device-position-updated = Position des Geräts {$device_id} auf ({$x}, {$y}, {$z}) aktualisiert
device-blink-command = Blink-Befehl an Gerät {$device_id} gesendet: {$state}

# General
loading = Lädt...
new-world-name = NeueWelt-{$timestamp}
new-world-description = Eine neue Welt
