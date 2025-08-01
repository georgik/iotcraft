# Main Menu
menu-enter-world = Entrer dans le monde
menu-quit-application = Quitter l'application
menu-select-world = Sélectionner un monde
menu-create-new-world = Créer un nouveau monde
menu-return-to-game = Retourner au jeu
menu-save-and-quit = Sauvegarder et quitter vers le menu principal
menu-quit-no-save = Quitter vers le menu principal (sans sauvegarder)

# World Selection
world-last-played = Dernière fois joué : {$time}
world-unknown-time = Inconnu

# Inventory and Items
item-grass = Herbe
item-dirt = Terre
item-stone = Pierre
item-quartz-block = Bloc de quartz
item-glass-pane = Vitre
item-cyan-terracotta = Terre cuite cyan
inventory-empty = Vide

# Console Commands
console-blink-started = Clignotement démarré
console-blink-stopped = Clignotement arrêté
console-blink-usage = Usage : blink [start|stop]
console-mqtt-connected = Connecté au courtier MQTT
console-mqtt-connecting = Connexion au courtier MQTT...
console-mqtt-temperature = Température actuelle : {$temp}°C
console-mqtt-no-temperature = Aucune donnée de température disponible
console-mqtt-usage = Usage : mqtt [status|temp]
console-placed-block = Bloc {$block_type} placé à ({$x}, {$y}, {$z})
console-removed-block = Bloc supprimé à ({$x}, {$y}, {$z})
console-no-block-found = Aucun bloc trouvé à ({$x}, {$y}, {$z})
console-teleported = Téléporté à ({$x}, {$y}, {$z})
console-look-set = Angles de vue définis sur yaw : {$yaw}°, pitch : {$pitch}°
console-map-saved = Carte sauvegardée dans '{$filename}' avec {$count} blocs
console-map-loaded = Carte chargée depuis '{$filename}' avec {$count} blocs
console-map-save-failed = Échec de la sauvegarde de la carte : {$error}
console-map-load-failed = Échec du chargement de la carte : {$error}
console-script-loaded = {$count} commandes chargées depuis {$filename}
console-script-load-failed = Erreur lors du chargement du script {$filename} : {$error}
console-spawn-sent = Commande spawn envoyée pour l'appareil {$device_id}
console-wall-created = Mur créé en {$block_type} de ({$x1}, {$y1}, {$z1}) à ({$x2}, {$y2}, {$z2})
console-gave-items = Ajouté {$quantity} x {$item_type}
console-invalid-block-type = Type de bloc invalide : {$block_type}
console-invalid-item-type = Type d'objet invalide : {$item_type}
console-unknown-command = Commande inconnue : {$command}

# Diagnostics
debug-title = Informations de débogage IoTCraft (Appuyez sur F3 pour basculer)
debug-divider = ------------------------------------------------------------------------------------------
debug-player-info = - INFORMATIONS DU JOUEUR
debug-position = Position : X={$x}  Y={$y}  Z={$z}
debug-rotation = Rotation : Yaw={$yaw}°  Pitch={$pitch}°
debug-selected-slot = Emplacement sélectionné : {$slot} ({$item})
debug-world-info = - INFORMATIONS DU MONDE
debug-total-blocks = Nombre total de blocs : {$count}
debug-iot-devices = Appareils IoT : {$count}
debug-session-time = Temps de session : {$minutes}m {$seconds}s
debug-script-commands = - COMMANDES DE SCRIPT
debug-teleport = Téléportation : tp {$x} {$y} {$z}
debug-look-direction = Direction du regard : look {$yaw} {$pitch}
debug-controls = - CONTRÔLES
debug-f3-toggle = F3 : Basculer cet écran de débogage
debug-console-open = T : Ouvrir la console
debug-inventory-select = 1-9 : Sélectionner un emplacement d'inventaire
debug-inventory-scroll = Molette de la souris : Faire défiler les emplacements d'inventaire

# Error Messages
error-camera-not-found = Erreur : Impossible de trouver la caméra
error-camera-teleport-failed = Erreur : Impossible de trouver la caméra pour la téléportation
error-camera-look-failed = Erreur : Impossible de trouver la caméra pour définir la direction du regard

# Device Messages
device-announce = Appareil {$device_id} annoncé
device-position-updated = Position de l'appareil {$device_id} mise à jour à ({$x}, {$y}, {$z})
device-blink-command = Commande blink envoyée à l'appareil {$device_id} : {$state}

# General
loading = Chargement...
new-world-name = NouveauMonde-{$timestamp}
new-world-description = Un nouveau monde
