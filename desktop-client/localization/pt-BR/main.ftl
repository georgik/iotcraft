# Main Menu
menu-enter-world = Entrar no mundo
menu-quit-application = Sair do aplicativo
menu-select-world = Selecionar mundo
menu-create-new-world = Criar novo mundo
menu-return-to-game = Retornar ao jogo
menu-save-and-quit = Salvar e sair para o menu principal
menu-quit-no-save = Sair para o menu principal (sem salvar)
menu-language = Idioma:
menu-settings = Configurações
menu-back-to-main = Voltar ao menu principal

# World Selection
world-last-played = Última vez jogado: {$time}
world-unknown-time = Desconhecido

# Inventory and Items
item-grass = Grama
item-dirt = Terra
item-stone = Pedra
item-quartz-block = Bloco de quartzo
item-glass-pane = Vitraça
item-cyan-terracotta = Terracota ciano
inventory-empty = Vazio

# Console Commands
console-blink-started = Piscar iniciado
console-blink-stopped = Piscar parado
console-blink-usage = Uso: piscar [iniciar|parar]
console-mqtt-connected = Conectado ao broker MQTT
console-mqtt-connecting = Conectando ao broker MQTT...
console-mqtt-temperature = Temperatura atual: {$temp}°C
console-mqtt-no-temperature = Nenhum dado de temperatura disponível
console-mqtt-usage = Uso: mqtt [status|temp]
console-placed-block = Bloco {$block_type} colocado em ({$x}, {$y}, {$z})
console-removed-block = Bloco removido em ({$x}, {$y}, {$z})
console-no-block-found = Nenhum bloco encontrado em ({$x}, {$y}, {$z})
console-teleported = Teletransportado para ({$x}, {$y}, {$z})
console-look-set = Ângulos de visão definidos como yaw: {$yaw}°, pitch: {$pitch}°
console-map-saved = Mapa salvo em '{$filename}' com {$count} blocos
console-map-loaded = Mapa carregado de '{$filename}' com {$count} blocos
console-map-save-failed = Falha ao salvar mapa: {$error}
console-map-load-failed = Falha ao carregar mapa: {$error}
console-script-loaded = {$count} comandos carregados de {$filename}
console-script-load-failed = Erro ao carregar script {$filename}: {$error}
console-spawn-sent = Comando de spawn enviado para o dispositivo {$device_id}
console-wall-created = Parede criada em {$block_type} de ({$x1}, {$y1}, {$z1}) para ({$x2}, {$y2}, {$z2})
console-gave-items = Adicionado {$quantity} x {$item_type}
console-invalid-block-type = Tipo de bloco inválido: {$block_type}
console-invalid-item-type = Tipo de item inválido: {$item_type}
console-unknown-command = Comando desconhecido: {$command}

# Diagnostics
debug-title = Informações de depuração do IoTCraft (Pressione F3 para alternar)
debug-divider = ------------------------------------------------------------------------------------------
debug-player-info = - INFORMAÇÕES DO JOGADOR
debug-position = Posição: X={$x}  Y={$y}  Z={$z}
debug-rotation = Rotação: Yaw={$yaw}°  Pitch={$pitch}°
debug-selected-slot = Slot selecionado: {$slot} ({$item})
debug-world-info = - INFORMAÇÕES DO MUNDO
debug-total-blocks = Total de blocos: {$count}
debug-iot-devices = Dispositivos IoT: {$count}
debug-session-time = Tempo de sessão: {$minutes}m {$seconds}s
debug-script-commands = - COMANDOS DE SCRIPT
debug-teleport = Teletransportar: tp {$x} {$y} {$z}
debug-look-direction = Direção do olhar: olhar {$yaw} {$pitch}
debug-controls = - CONTROLES
debug-f3-toggle = F3: Alternar esta tela de depuração
debug-console-open = T: Abrir console
debug-inventory-select = 1-9: Selecionar slot de inventário
debug-inventory-scroll = Roda do mouse: Rolagem dos slots do inventário

# Error Messages
error-camera-not-found = Erro: Não foi possível encontrar a câmera
error-camera-teleport-failed = Erro: Não foi possível encontrar a câmera para teletransporte
error-camera-look-failed = Erro: Não foi possível encontrar a câmera para definir a direção do olhar

# Device Messages
device-announce = Dispositivo {$device_id} anunciado

# General
loading = Carregando...
new-world-name = NovoMundo-{$timestamp}
new-world-description = Um novo mundo
