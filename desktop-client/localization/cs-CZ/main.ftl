# Main Menu
menu-enter-world = Vstoupit do světa
menu-quit-application = Ukončit aplikaci
menu-select-world = Vybrat svět
menu-create-new-world = Vytvořit nový svět
menu-return-to-game = Vrátit se do hry
menu-save-and-quit = Uložit a ukončit do hlavního menu
menu-quit-no-save = Ukončit do hlavního menu (bez uložení)
menu-language = Jazyk:
menu-settings = Nastavení
menu-back-to-main = Zpět do hlavního menu

# World Selection
world-last-played = Naposledy hráno: {$time}
world-unknown-time = Neznámé

# Inventory and Items
item-grass = Tráva
item-dirt = Hlína
item-stone = Kámen
item-quartz-block = Křemenný blok
item-glass-pane = Skleněná tabule
item-cyan-terracotta = Tyrkysová terakota
inventory-empty = Prázdné

# Console Commands
console-blink-started = Blikání spuštěno
console-blink-stopped = Blikání zastaveno
console-blink-usage = Použití: blink [start|stop]
console-mqtt-connected = Připojeno k MQTT brokeru
console-mqtt-connecting = Připojování k MQTT brokeru...
console-mqtt-temperature = Aktuální teplota: {$temp}°C
console-mqtt-no-temperature = Žádná data o teplotě nejsou k dispozici
console-mqtt-usage = Použití: mqtt [status|temp]
console-placed-block = Umístěn blok {$block_type} na ({$x}, {$y}, {$z})
console-removed-block = Odstraněn blok na ({$x}, {$y}, {$z})
console-no-block-found = Žádný blok nebyl nalezen na ({$x}, {$y}, {$z})
console-teleported = Teleportováno na ({$x}, {$y}, {$z})
console-look-set = Nastaveny úhly pohledu na yaw: {$yaw}°, pitch: {$pitch}°
console-map-saved = Mapa uložena do '{$filename}' s {$count} bloky
console-map-loaded = Mapa načtena z '{$filename}' s {$count} bloky
console-map-save-failed = Nepodařilo se uložit mapu: {$error}
console-map-load-failed = Nepodařilo se načíst mapu: {$error}
console-script-loaded = Načteno {$count} příkazů z {$filename}
console-script-load-failed = Chyba při načítání skriptu {$filename}: {$error}
console-spawn-sent = Spawn příkaz odeslán pro zařízení {$device_id}
console-wall-created = Vytvořena zeď z {$block_type} od ({$x1}, {$y1}, {$z1}) do ({$x2}, {$y2}, {$z2})
console-gave-items = Přidáno {$quantity} x {$item_type}
console-invalid-block-type = Neplatný typ bloku: {$block_type}
console-invalid-item-type = Neplatný typ předmětu: {$item_type}
console-unknown-command = Neznámý příkaz: {$command}

# Diagnostics
debug-title = IoTCraft ladicí informace (Stiskněte F3 pro přepnutí)
debug-divider = ------------------------------------------------------------------------------------------
debug-player-info = - INFORMACE O HRÁČI
debug-position = Pozice: X={$x}  Y={$y}  Z={$z}
debug-rotation = Rotace: Yaw={$yaw}°  Pitch={$pitch}°
debug-selected-slot = Vybraný slot: {$slot} ({$item})
debug-world-info = - INFORMACE O SVĚTĚ
debug-total-blocks = Celkový počet bloků: {$count}
debug-iot-devices = IoT zařízení: {$count}
debug-session-time = Čas relace: {$minutes}m {$seconds}s
debug-script-commands = - SKRIPT PŘÍKAZY
debug-teleport = Teleport: tp {$x} {$y} {$z}
debug-look-direction = Směr pohledu: look {$yaw} {$pitch}
debug-controls = - OVLÁDÁNÍ
debug-f3-toggle = F3: Přepnout tuto obrazovku ladění
debug-console-open = T: Otevřít konzoli
debug-inventory-select = 1-9: Vybrat slot inventáře
debug-inventory-scroll = Kolečko myši: Procházet sloty inventáře

# Error Messages
error-camera-not-found = Chyba: Nelze najít kameru
error-camera-teleport-failed = Chyba: Nelze najít kameru pro teleport
error-camera-look-failed = Chyba: Nelze najít kameru pro nastavení směru pohledu

# Device Messages
device-announce = Zařízení {$device_id} oznámeno
device-position-updated = Pozice zařízení {$device_id} aktualizována na ({$x}, {$y}, {$z})
device-blink-command = Blink příkaz odeslán zařízení {$device_id}: {$state}

# General
loading = Načítání...
new-world-name = NovýSvět-{$timestamp}
new-world-description = Nový svět
