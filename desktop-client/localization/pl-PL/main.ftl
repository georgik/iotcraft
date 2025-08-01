# Main Menu
menu-enter-world = Wejdź do świata
menu-quit-application = Zakończ aplikację
menu-select-world = Wybierz świat
menu-create-new-world = Utwórz nowy świat
menu-return-to-game = Powróć do gry
menu-save-and-quit = Zapisz i wyjdź do menu głównego
menu-quit-no-save = Wyjdź do menu głównego (bez zapisywania)

# World Selection
world-last-played = Ostatnio grane: {$time}
world-unknown-time = Nieznane

# Inventory and Items
item-grass = Trawa
item-dirt = Ziemia
item-stone = Kamień
item-quartz-block = Blok kwarcu
item-glass-pane = Szybka
item-cyan-terracotta = Cyjanowa terakota
inventory-empty = Puste

# Console Commands
console-blink-started = Miganie rozpoczęte
console-blink-stopped = Miganie zatrzymane
console-blink-usage = Użycie: blink [start|stop]
console-mqtt-connected = Połączono z brokerem MQTT
console-mqtt-connecting = Łączenie z brokerem MQTT...
console-mqtt-temperature = Aktualna temperatura: {$temp}°C
console-mqtt-no-temperature = Brak danych o temperaturze
console-mqtt-usage = Użycie: mqtt [status|temp]
console-placed-block = Umieszczono blok {$block_type} na ({$x}, {$y}, {$z})
console-removed-block = Usunięto blok na ({$x}, {$y}, {$z})
console-no-block-found = Nie znaleziono bloku na ({$x}, {$y}, {$z})
console-teleported = Teleportowano do ({$x}, {$y}, {$z})
console-look-set = Ustawiono kąty widoku na yaw: {$yaw}°, pitch: {$pitch}°
console-map-saved = Mapa zapisana do '{$filename}' z {$count} blokami
console-map-loaded = Mapa wczytana z '{$filename}' z {$count} blokami
console-map-save-failed = Nie udało się zapisać mapy: {$error}
console-map-load-failed = Nie udało się wczytać mapy: {$error}
console-script-loaded = Wczytano {$count} poleceń z {$filename}
console-script-load-failed = Błąd wczytywania skryptu {$filename}: {$error}
console-spawn-sent = Polecenie spawn wysłane dla urządzenia {$device_id}
console-wall-created = Utworzono ścianę z {$block_type} od ({$x1}, {$y1}, {$z1}) do ({$x2}, {$y2}, {$z2})
console-gave-items = Dodano {$quantity} x {$item_type}
console-invalid-block-type = Nieprawidłowy typ bloku: {$block_type}
console-invalid-item-type = Nieprawidłowy typ przedmiotu: {$item_type}
console-unknown-command = Nieznane polecenie: {$command}

# Diagnostics
debug-title = Informacje debugowania IoTCraft (Naciśnij F3, aby przełączyć)
debug-divider = ------------------------------------------------------------------------------------------
debug-player-info = - INFORMACJE O GRACZU
debug-position = Pozycja: X={$x}  Y={$y}  Z={$z}
debug-rotation = Obrót: Yaw={$yaw}°  Pitch={$pitch}°
debug-selected-slot = Wybrany slot: {$slot} ({$item})
debug-world-info = - INFORMACJE O ŚWIECIE
debug-total-blocks = Łączna liczba bloków: {$count}
debug-iot-devices = Urządzenia IoT: {$count}
debug-session-time = Czas sesji: {$minutes}m {$seconds}s
debug-script-commands = - POLECENIA SKRYPTU
debug-teleport = Teleport: tp {$x} {$y} {$z}
debug-look-direction = Kierunek patrzenia: look {$yaw} {$pitch}
debug-controls = - STEROWANIE
debug-f3-toggle = F3: Przełącz ten ekran debugowania
debug-console-open = T: Otwórz konsolę
debug-inventory-select = 1-9: Wybierz slot ekwipunku
debug-inventory-scroll = Kółko myszy: Przewijaj sloty ekwipunku

# Error Messages
error-camera-not-found = Błąd: Nie można znaleźć kamery
error-camera-teleport-failed = Błąd: Nie można znaleźć kamery do teleportacji
error-camera-look-failed = Błąd: Nie można znaleźć kamery do ustawienia kierunku patrzenia

# Device Messages
device-announce = Urządzenie {$device_id} ogłoszone
device-position-updated = Pozycja urządzenia {$device_id} zaktualizowana do ({$x}, {$y}, {$z})
device-blink-command = Polecenie blink wysłane do urządzenia {$device_id}: {$state}

# General
loading = Ładowanie...
new-world-name = NowyŚwiat-{$timestamp}
new-world-description = Nowy świat
