# Main Menu
menu-enter-world = Vstúpiť do sveta
menu-quit-application = Ukončiť aplikáciu
menu-select-world = Vybrať svet
menu-create-new-world = Vytvoriť nový svet
menu-return-to-game = Vrátiť sa do hry
menu-save-and-quit = Uložiť a ukončiť do hlavného menu
menu-quit-no-save = Ukončiť do hlavného menu (bez uloženia)

# World Selection
world-last-played = Naposledy hrané: {$time}
world-unknown-time = Neznáme

# Inventory and Items
item-grass = Tráva
item-dirt = Hlina
item-stone = Kameň
item-quartz-block = Kremeňový blok
item-glass-pane = Sklenená tabuľa
item-cyan-terracotta = Tyrkysová terakota
inventory-empty = Prázdne

# Console Commands
console-blink-started = Blikanie spustené
console-blink-stopped = Blikanie zastavené
console-blink-usage = Použitie: blink [start|stop]
console-mqtt-connected = Pripojené k MQTT brokeru
console-mqtt-connecting = Pripájanie k MQTT brokeru...
console-mqtt-temperature = Aktuálna teplota: {$temp}°C
console-mqtt-no-temperature = Žiadne údaje o teplote k dispozícii
console-mqtt-usage = Použitie: mqtt [status|temp]
console-placed-block = Umiestený blok {$block_type} na ({$x}, {$y}, {$z})
console-removed-block = Odstránený blok na ({$x}, {$y}, {$z})
console-no-block-found = Žiadny blok nebol nájdený na ({$x}, {$y}, {$z})
console-teleported = Teleportované na ({$x}, {$y}, {$z})
console-look-set = Nastavené uhly pohľadu na yaw: {$yaw}°, pitch: {$pitch}°
console-map-saved = Mapa uložená do '{$filename}' s {$count} blokmi
console-map-loaded = Mapa načítaná z '{$filename}' s {$count} blokmi
console-map-save-failed = Nepodarilo sa uložiť mapu: {$error}
console-map-load-failed = Nepodarilo sa načítať mapu: {$error}
console-script-loaded = Načítaných {$count} príkazov z {$filename}
console-script-load-failed = Chyba pri načítaní skriptu {$filename}: {$error}
console-spawn-sent = Spawn príkaz odoslaný pre zariadenie {$device_id}
console-wall-created = Vytvorená stena z {$block_type} od ({$x1}, {$y1}, {$z1}) do ({$x2}, {$y2}, {$z2})
console-gave-items = Pridané {$quantity} x {$item_type}
console-invalid-block-type = Neplatný typ bloku: {$block_type}
console-invalid-item-type = Neplatný typ položky: {$item_type}
console-unknown-command = Neznámy príkaz: {$command}

# Diagnostics
debug-title = IoTCraft ladenie informácií (Stlačte F3 pre prepnutie)
debug-divider = ------------------------------------------------------------------------------------------
debug-player-info = - INFORMÁCIE O HRÁČOVI
debug-position = Poloha: X={$x}  Y={$y}  Z={$z}
debug-rotation = Rotácia: Yaw={$yaw}°  Pitch={$pitch}°
debug-selected-slot = Vybraný slot: {$slot} ({$item})
debug-world-info = - INFORMÁCIE O SVETE
debug-total-blocks = Celkový počet blokov: {$count}
debug-iot-devices = IoT zariadenia: {$count}
debug-session-time = Čas relácie: {$minutes}m {$seconds}s
debug-script-commands = - SKRIPT PRÍKAZY
debug-teleport = Teleport: tp {$x} {$y} {$z}
debug-look-direction = Smer pohľadu: look {$yaw} {$pitch}
debug-controls = - OVLÁDANIE
debug-f3-toggle = F3: Prepnúť túto obrazovku ladenia
debug-console-open = T: Otvoriť konzolu
debug-inventory-select = 1-9: Vybrať slot inventára
debug-inventory-scroll = Koliesko myši: Prechádzať sloty inventára

# Error Messages
error-camera-not-found = Chyba: Nie je možné nájsť kameru
error-camera-teleport-failed = Chyba: Nie je možné nájsť kameru na teleport
error-camera-look-failed = Chyba: Nie je možné nájsť kameru na nastavenie smeru pohľadu

# Device Messages
device-announce = Zariadenie {$device_id} oznámené
device-position-updated = Poloha zariadenia {$device_id} aktualizovaná na ({$x}, {$y}, {$z})
device-blink-command = Blink príkaz odoslaný zariadeniu {$device_id}: {$state}

# General
loading = Načítanie...
new-world-name = NovýSvet-{$timestamp}
new-world-description = Nový svet
