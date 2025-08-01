# Main Menu
menu-enter-world = 进入世界
menu-quit-application = 退出应用程序
menu-select-world = 选择一个世界
menu-create-new-world = 创建新世界
menu-return-to-game = 返回游戏
menu-save-and-quit = 保存并退出到主菜单
menu-quit-no-save = 退出到主菜单 (不保存)

# World Selection
world-last-played = 上次游戏： {$time}
world-unknown-time = 未知

# Inventory and Items
item-grass = 草
item-dirt = 土
item-stone = 石头
item-quartz-block = 石英块
item-glass-pane = 玻璃板
item-cyan-terracotta = 青瓷砖
inventory-empty = 空的

# Console Commands
console-blink-started = Blink started
console-blink-stopped = Blink stopped
console-blink-usage = Usage: blink [start|stop]
console-mqtt-connected = Connected to MQTT broker
console-mqtt-connecting = Connecting to MQTT broker...
console-mqtt-temperature = Current temperature: {$temp}°C
console-mqtt-no-temperature = No temperature data available
console-mqtt-usage = Usage: mqtt [status|temp]
console-placed-block = Placed {$block_type} block at ({$x}, {$y}, {$z})
console-removed-block = Removed block at ({$x}, {$y}, {$z})
console-no-block-found = No block found at ({$x}, {$y}, {$z})
console-teleported = Teleported to ({$x}, {$y}, {$z})
console-look-set = Set look angles to yaw: {$yaw}°, pitch: {$pitch}°
console-map-saved = Map saved to '{$filename}' with {$count} blocks
console-map-loaded = Map loaded from '{$filename}' with {$count} blocks
console-map-save-failed = Failed to save map: {$error}
console-map-load-failed = Failed to load map: {$error}
console-script-loaded = Loaded {$count} commands from {$filename}
console-script-load-failed = Error loading script {$filename}: {$error}
console-spawn-sent = Spawn command sent for device {$device_id}
console-wall-created = Created a wall of {$block_type} from ({$x1}, {$y1}, {$z1}) to ({$x2}, {$y2}, {$z2})
console-gave-items = Added {$quantity} x {$item_type}
console-invalid-block-type = Invalid block type: {$block_type}
console-invalid-item-type = Invalid item type: {$item_type}
console-unknown-command = Unknown command: {$command}

# Diagnostics
debug-title = IoTCraft Debug Information (Press F3 to toggle)
debug-divider = ------------------------------------------------------------------------------------------
debug-player-info = - PLAYER INFORMATION
debug-position = Position: X={$x}  Y={$y}  Z={$z}
debug-rotation = Rotation: Yaw={$yaw}°  Pitch={$pitch}°
debug-selected-slot = Selected Slot: {$slot} ({$item})
debug-world-info = - WORLD INFORMATION
debug-total-blocks = Total Blocks: {$count}
debug-iot-devices = IoT Devices: {$count}
debug-session-time = Session Time: {$minutes}m {$seconds}s
debug-script-commands = - SCRIPT COMMANDS
debug-teleport = Teleport: tp {$x} {$y} {$z}
debug-look-direction = Look Direction: look {$yaw} {$pitch}
debug-controls = - CONTROLS
debug-f3-toggle = F3: Toggle this debug screen
debug-console-open = T: Open console
debug-inventory-select = 1-9: Select inventory slot
debug-inventory-scroll = Mouse Wheel: Scroll inventory slots

# Error Messages
error-camera-not-found = Error: Could not find camera
error-camera-teleport-failed = Error: Could not find camera to teleport
error-camera-look-failed = Error: Could not find camera to set look direction

# Device Messages
device-announce = Device {$device_id} announced
device-position-updated = Device {$device_id} position updated to ({$x}, {$y}, {$z})
device-blink-command = Blink command sent to device {$device_id}: {$state}

# General
loading = Loading...
new-world-name = NewWorld-{$timestamp}
new-world-description = A new world
