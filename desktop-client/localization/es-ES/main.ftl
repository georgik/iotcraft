# Main Menu
menu-enter-world = Entrar al mundo
menu-quit-application = Salir de la aplicación
menu-select-world = Seleccionar un mundo
menu-create-new-world = Crear nuevo mundo
menu-return-to-game = Volver al juego
menu-save-and-quit = Guardar y salir al menú principal
menu-quit-no-save = Salir al menú principal (sin guardar)

# World Selection
world-last-played = Última vez jugado: {$time}
world-unknown-time = Desconocido

# Inventory and Items
item-grass = Hierba
item-dirt = Tierra
item-stone = Piedra
item-quartz-block = Bloque de cuarzo
item-glass-pane = Panel de cristal
item-cyan-terracotta = Terracota cian
inventory-empty = Vacío

# Console Commands
console-blink-started = Parpadeo iniciado
console-blink-stopped = Parpadeo detenido
console-blink-usage = Uso: blink [start|stop]
console-mqtt-connected = Conectado al broker MQTT
console-mqtt-connecting = Conectando al broker MQTT...
console-mqtt-temperature = Temperatura actual: {$temp}°C
console-mqtt-no-temperature = No hay datos de temperatura disponibles
console-mqtt-usage = Uso: mqtt [status|temp]
console-placed-block = Colocado bloque de {$block_type} en ({$x}, {$y}, {$z})
console-removed-block = Eliminado bloque en ({$x}, {$y}, {$z})
console-no-block-found = No se encontró bloque en ({$x}, {$y}, {$z})
console-teleported = Teletransportado a ({$x}, {$y}, {$z})
console-look-set = Ángulos de vista establecidos a yaw: {$yaw}°, pitch: {$pitch}°
console-map-saved = Mapa guardado en '{$filename}' con {$count} bloques
console-map-loaded = Mapa cargado desde '{$filename}' con {$count} bloques
console-map-save-failed = Error al guardar el mapa: {$error}
console-map-load-failed = Error al cargar el mapa: {$error}
console-script-loaded = Cargados {$count} comandos desde {$filename}
console-script-load-failed = Error cargando script {$filename}: {$error}
console-spawn-sent = Comando spawn enviado para dispositivo {$device_id}
console-wall-created = Creada pared de {$block_type} desde ({$x1}, {$y1}, {$z1}) hasta ({$x2}, {$y2}, {$z2})
console-gave-items = Añadidos {$quantity} x {$item_type}
console-invalid-block-type = Tipo de bloque inválido: {$block_type}
console-invalid-item-type = Tipo de objeto inválido: {$item_type}
console-unknown-command = Comando desconocido: {$command}

# Diagnostics
debug-title = Información de depuración de IoTCraft (Presiona F3 para alternar)
debug-divider = ------------------------------------------------------------------------------------------
debug-player-info = - INFORMACIÓN DEL JUGADOR
debug-position = Posición: X={$x}  Y={$y}  Z={$z}
debug-rotation = Rotación: Yaw={$yaw}°  Pitch={$pitch}°
debug-selected-slot = Ranura seleccionada: {$slot} ({$item})
debug-world-info = - INFORMACIÓN DEL MUNDO
debug-total-blocks = Bloques totales: {$count}
debug-iot-devices = Dispositivos IoT: {$count}
debug-session-time = Tiempo de sesión: {$minutes}m {$seconds}s
debug-script-commands = - COMANDOS DE SCRIPT
debug-teleport = Teletransporte: tp {$x} {$y} {$z}
debug-look-direction = Dirección de vista: look {$yaw} {$pitch}
debug-controls = - CONTROLES
debug-f3-toggle = F3: Alternar esta pantalla de depuración
debug-console-open = T: Abrir consola
debug-inventory-select = 1-9: Seleccionar ranura de inventario
debug-inventory-scroll = Rueda del ratón: Desplazar ranuras del inventario

# Error Messages
error-camera-not-found = Error: No se pudo encontrar la cámara
error-camera-teleport-failed = Error: No se pudo encontrar la cámara para teletransportar
error-camera-look-failed = Error: No se pudo encontrar la cámara para establecer la dirección de vista

# Device Messages
device-announce = Dispositivo {$device_id} anunciado
device-position-updated = Posición del dispositivo {$device_id} actualizada a ({$x}, {$y}, {$z})
device-blink-command = Comando de parpadeo enviado al dispositivo {$device_id}: {$state}

# General
loading = Cargando...
new-world-name = NuevoMundo-{$timestamp}
new-world-description = Un nuevo mundo
