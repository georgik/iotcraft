
// IoTCraft World View Types
enum BlockType {
    case grass
    case dirt
    case stone
    case quartzBlock
    case glassPane
    case cyanTerracotta
}

enum DeviceType {
    case lamp
    case door
    case sensor
}

// World Block
struct WorldBlock {
    var x: Int
    var y: Int
    var z: Int
    var blockType: BlockType
}

// IoT Device
struct IoTDevice {
    var id: String
    var x: Float
    var y: Float
    var z: Float
    var deviceType: DeviceType
    var isOnline: Bool
    var lightState: Bool // for lamps
}

// Camera/View state
struct WorldCamera {
    var centerX: Float
    var centerZ: Float
    var zoom: Float
    var viewWidth: Float
    var viewHeight: Float
}


@_cdecl("app_main")
func app_main() {
    print("Initializing SDL3 from Swift.")

    // Initialize pthread attributes
    var sdl_pthread = pthread_t(bitPattern: 0)
    var attr = pthread_attr_t()

    pthread_attr_init(&attr)
    pthread_attr_setstacksize(&attr, 32000) // Set the stack size for the thread

    // Create the SDL thread
    let ret = pthread_create(&sdl_pthread, &attr, sdl_thread_entry_point, nil)
    if ret != 0 {
        print("Failed to create SDL thread")
        return
    }

    // Optionally detach the thread if you don't need to join it later
    pthread_detach(sdl_pthread)
}

func pointInRect(x: Float, y: Float, rect: SDL_FRect) -> Bool {
    let margin: Float = 30.0
    return x >= rect.x - margin && x <= rect.x + rect.w + margin &&
           y >= rect.y - margin && y <= rect.y + rect.h + margin
}

// Function to generate a random Float between min and max
func getRandomFloat(min: Float, max: Float) -> Float {
    let scale = Float.random(in: 0...1)
    return min + scale * (max - min)
}

// Generate sample IoTCraft world
func generateSampleWorld() -> ([WorldBlock], [IoTDevice]) {
    var blocks: [WorldBlock] = []
    var devices: [IoTDevice] = []
    
    // Create a simple world layout (similar to the Swift client)
    let worldData: [(Int, Int, Int, BlockType)] = [
        (0, 0, 0, .grass), (1, 0, 0, .grass), (2, 0, 0, .stone),
        (0, 0, 1, .dirt), (1, 0, 1, .dirt), (2, 0, 1, .stone),
        (-1, 0, 0, .grass), (-1, 0, 1, .dirt), (-2, 0, 2, .cyanTerracotta),
        (3, 0, 0, .quartzBlock), (3, 0, 1, .glassPane),
        (0, 0, 2, .stone), (1, 0, 2, .stone), (2, 0, 2, .quartzBlock)
    ]
    
    for (x, y, z, blockType) in worldData {
        blocks.append(WorldBlock(x: x, y: y, z: z, blockType: blockType))
    }
    
    // Add some IoT devices
    devices.append(IoTDevice(
        id: "esp32_lamp_001",
        x: 0.5, y: 1.0, z: 0.5,
        deviceType: .lamp,
        isOnline: true,
        lightState: true
    ))
    
    devices.append(IoTDevice(
        id: "esp32_door_001", 
        x: 2.5, y: 1.0, z: 1.5,
        deviceType: .door,
        isOnline: true,
        lightState: false
    ))
    
    devices.append(IoTDevice(
        id: "esp32_sensor_001",
        x: -1.5, y: 1.0, z: 2.5,
        deviceType: .sensor,
        isOnline: true,
        lightState: false
    ))
    
    return (blocks, devices)
}

// Convert world coordinates to screen coordinates
func worldToScreen(worldX: Float, worldZ: Float, camera: WorldCamera) -> (Float, Float) {
    let blockSize: Float = 40.0 * camera.zoom
    
    let screenX = (camera.viewWidth / 2.0) + (worldX - camera.centerX) * blockSize
    let screenY = (camera.viewHeight / 2.0) + (worldZ - camera.centerZ) * blockSize
    
    return (screenX, screenY)
}

// Get block color based on type
func getBlockColor(blockType: BlockType) -> (UInt8, UInt8, UInt8) {
    switch blockType {
    case .grass: return (34, 139, 34)      // Forest Green
    case .dirt: return (139, 69, 19)       // Saddle Brown
    case .stone: return (128, 128, 128)    // Gray
    case .quartzBlock: return (245, 245, 220) // Beige
    case .glassPane: return (173, 216, 230)   // Light Blue
    case .cyanTerracotta: return (95, 158, 160) // Cadet Blue
    }
}

// Get device color based on type and state
func getDeviceColor(device: IoTDevice) -> (UInt8, UInt8, UInt8) {
    if !device.isOnline {
        return (64, 64, 64) // Dark gray for offline
    }
    
    switch device.deviceType {
    case .lamp:
        return device.lightState ? (255, 255, 0) : (128, 128, 0) // Yellow or dim yellow
    case .door:
        return (139, 69, 19) // Brown
    case .sensor:
        return (0, 255, 0) // Green
    }
}

var scoreDestRect = SDL_FRect(x: 10.0, y: 10.0, w: 120.0, h: 50.0)
var score = 0


func sdl_thread_entry_point(arg: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    print("IoTCraft ESP32-P4 Tab5 Client started.")
    
    // Initialize IoTCraft world data
    var worldBlocks: [WorldBlock] = []
    var iotDevices: [IoTDevice] = []
    
    // Screen boundaries
    let screenWidth = Float(BSP_LCD_H_RES)  // 720 for Tab5
    let screenHeight = Float(BSP_LCD_V_RES) // 1280 for Tab5
    
    // Initialize world camera for top-down view
    var camera = WorldCamera(
        centerX: 0.0,
        centerZ: 0.0, 
        zoom: 1.0,
        viewWidth: screenWidth,
        viewHeight: screenHeight
    )

    // Initialize SDL
    if SDL_Init(UInt32(SDL_INIT_VIDEO | SDL_INIT_EVENTS)) == false {
        print("Unable to initialize SDL")
        return nil
    }
    print("SDL initialized successfully")

    guard let window = SDL_CreateWindow(nil, Int32(BSP_LCD_H_RES), Int32(BSP_LCD_V_RES), 0) else {
        return nil
    }

    var width: Int32 = 0
    var height: Int32 = 0

    // Get window size
    SDL_GetWindowSize(window, &width, &height)

    // Print the resolution
    print("Display resolution: 720x1280")

    // Create SDL renderer
    guard let renderer = SDL_CreateRenderer(window, nil) else {
        print("Failed to create renderer")
        return nil
    }

    SDL_SetRenderDrawColor(renderer, 22, 10, 33, 255)
    SDL_RenderClear(renderer)
    SDL_RenderPresent(renderer)

    SDL_InitFS();

    // TTF_Init() // Disabled SDL_ttf dependency
    // let font = TTF_OpenFont("/assets/FreeSans.ttf", 36);
    // if (font == nil) {
    //     print("Font load failed")
    // }

    // Generate the IoTCraft world
    let (blocks, devices) = generateSampleWorld()
    worldBlocks = blocks
    iotDevices = devices
    
    print("Generated IoTCraft world with blocks and devices")

    // Initialize IoTCraft world state
    var lastDeviceUpdate: UInt64 = SDL_GetTicksNS()
    var deviceUpdateInterval: UInt64 = 2_000_000_000 // 2 seconds in nanoseconds
    var event = SDL_Event()
    var running = true

    print("Entering main loop...")

    while running {
        let currentTime = SDL_GetTicksNS()
        
        // Handle events
        while SDL_PollEvent(&event) {
            if event.type == SDL_EVENT_QUIT.rawValue {
                running = false
                break
            } else if event.type == SDL_EVENT_FINGER_UP.rawValue {
                // Get touch coordinates (normalized between 0 and 1) and convert to screen coordinates
                let touchX = event.tfinger.x * screenWidth
                let touchY = event.tfinger.y * screenHeight
                
                print("Touch detected")
                
                // Check if touch intersects any device
                let blockSize: Float = 40.0 * camera.zoom
                for i in 0..<iotDevices.count {
                    let (screenX, screenY) = worldToScreen(worldX: iotDevices[i].x, worldZ: iotDevices[i].z, camera: camera)
                    
                    let deviceRect = SDL_FRect(
                        x: screenX - blockSize/2,
                        y: screenY - blockSize/2,
                        w: blockSize,
                        h: blockSize
                    )
                    
                    if pointInRect(x: touchX, y: touchY, rect: deviceRect) {
                        print("Device touched")
                        
                        // Toggle device state
                        if iotDevices[i].deviceType == .lamp {
                            iotDevices[i].lightState.toggle()
                            print("Lamp state toggled")
                        }
                        
                        break
                    }
                }
            }
        }

        // Update device states periodically (simulate real-time changes)
        if currentTime - lastDeviceUpdate > deviceUpdateInterval {
            // Simulate some device state changes
            for i in 0..<iotDevices.count {
                if iotDevices[i].deviceType == .lamp && Float.random(in: 0...1) < 0.3 {
                    iotDevices[i].lightState.toggle()
                    print("Device light state changed")
                }
            }
            lastDeviceUpdate = currentTime
        }

        // Clear the renderer with dark background
        SDL_SetRenderDrawColor(renderer, 20, 20, 30, 255)
        SDL_RenderClear(renderer)
        
        let blockSize: Float = 40.0 * camera.zoom
        
        // Render world blocks
        for block in worldBlocks {
            let (screenX, screenY) = worldToScreen(worldX: Float(block.x), worldZ: Float(block.z), camera: camera)
            
            // Skip blocks outside screen bounds
            if screenX < -blockSize || screenX > screenWidth + blockSize ||
               screenY < -blockSize || screenY > screenHeight + blockSize {
                continue
            }
            
            let (r, g, b) = getBlockColor(blockType: block.blockType)
            SDL_SetRenderDrawColor(renderer, r, g, b, 255)
            
            var blockRect = SDL_FRect(
                x: screenX - blockSize/2,
                y: screenY - blockSize/2,
                w: blockSize,
                h: blockSize
            )
            
            SDL_RenderFillRect(renderer, &blockRect)
            
            // Draw block outline
            SDL_SetRenderDrawColor(renderer, 64, 64, 64, 255)
            SDL_RenderRect(renderer, &blockRect)
        }
        
        // Render IoT devices
        for device in iotDevices {
            let (screenX, screenY) = worldToScreen(worldX: device.x, worldZ: device.z, camera: camera)
            
            // Skip devices outside screen bounds
            if screenX < -blockSize || screenX > screenWidth + blockSize ||
               screenY < -blockSize || screenY > screenHeight + blockSize {
                continue
            }
            
            let (r, g, b) = getDeviceColor(device: device)
            SDL_SetRenderDrawColor(renderer, r, g, b, 255)
            
            let deviceSize = blockSize * 0.8
            var deviceRect = SDL_FRect(
                x: screenX - deviceSize/2,
                y: screenY - deviceSize/2,
                w: deviceSize,
                h: deviceSize
            )
            
            SDL_RenderFillRect(renderer, &deviceRect)
            
            // Draw device outline (thicker for active devices)
            let outlineColor: (UInt8, UInt8, UInt8) = device.isOnline ? (255, 255, 255) : (128, 128, 128)
            SDL_SetRenderDrawColor(renderer, outlineColor.0, outlineColor.1, outlineColor.2, 255)
            SDL_RenderRect(renderer, &deviceRect)
            
            // Draw a smaller inner rect for lamps that are on
            if device.deviceType == .lamp && device.lightState {
                SDL_SetRenderDrawColor(renderer, 255, 255, 255, 200)
                let innerSize = deviceSize * 0.6
                var innerRect = SDL_FRect(
                    x: screenX - innerSize/2,
                    y: screenY - innerSize/2,
                    w: innerSize,
                    h: innerSize
                )
                SDL_RenderFillRect(renderer, &innerRect)
            }
        }

        // Count active devices
        let onlineDevices = iotDevices.filter { $0.isOnline }.count
        let activeLamps = iotDevices.filter { $0.deviceType == .lamp && $0.lightState }.count
        
        let statusText = "IoTCraft World | Blocks: \(worldBlocks.count) | Devices: \(onlineDevices) | Lamps: \(activeLamps)"

        // Use SDL's built-in debug text rendering instead of TTF
        var statusTextBuffer = Array(statusText.utf8CString)
        
        // Set text color to green
        SDL_SetRenderDrawColor(renderer, 40, 255, 40, 255)
        
        // Render debug text using SDL's built-in functionality
        SDL_RenderDebugText(renderer, 10.0, 10.0, statusTextBuffer)

        // Present the updated frame
        SDL_RenderPresent(renderer)

        // Delay to limit frame rate (~60 FPS)
        SDL_Delay(16)
        // print("tick")
    }
    return nil
}
