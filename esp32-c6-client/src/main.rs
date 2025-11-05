#![no_std]
#![no_main]
extern crate alloc;

// Add ESP-IDF App Descriptor - required for flashing
esp_bootloader_esp_idf::esp_app_desc!();
use crate::alloc::string::ToString;
use core::net::Ipv4Addr;
use log::{error, info};

use embassy_executor::Spawner;
use embassy_net::{Runner, Stack, StackResources, tcp::TcpSocket};
use embassy_time::{Duration, Timer};
use esp_alloc as _;
// use esp_backtrace as _;
use esp_hal::{clock::CpuClock, rng::Rng, timer::timg::TimerGroup};
use esp_println::{print, println};
use esp_radio::wifi::{
    ClientConfig as WifiClientConfig, ModeConfig, WifiController, WifiDevice, WifiEvent,
    WifiStaState,
};

#[cfg(feature = "neopixel")]
use esp_hal::rmt::Rmt;
#[cfg(feature = "neopixel")]
use esp_hal_smartled::{SmartLedsAdapter, smart_led_buffer};

#[cfg(feature = "neopixel")]
use smart_leds::{RGB8, SmartLedsWrite, brightness, gamma};

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
// TxChannel removed in esp-hal 1.0.0
// use rumqttc::{MqttOptions, Client, QoS};
// use rumqttc::{Event, Outgoing};

use rust_mqtt::{
    client::client::MqttClient, client::client_config::ClientConfig,
    packet::v5::publish_packet::QualityOfService, utils::rng_generator::CountingRng,
};

// Storage and serialization imports
use embedded_storage::{ReadStorage, Storage};
use esp_bootloader_esp_idf::partitions;
use esp_storage::FlashStorage;
use serde::{Deserialize, Serialize};
use serde_json_core;

// Define a static channel with a capacity of 1 for `HardwareEvent`s.
static CHANNEL: Channel<CriticalSectionRawMutex, HardwareEvent, 1> = Channel::new();
use core::sync::atomic::{AtomicBool, Ordering};
static IS_BUSY: AtomicBool = AtomicBool::new(false);

macro_rules! mk_static {
    ($t:ty, $val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[allow(unsafe_code, unused_unsafe)]
        unsafe {
            STATIC_CELL.init_with(|| $val)
        }
    }};
}

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");
const SERVER_IP: &str = env!("SERVER_IP");

// LED brightness (0-255)
const LED_BRIGHTNESS: u8 = 255;

// Storage constants for storage partition
const DEVICE_PROPERTIES_OFFSET: u32 = 0x0; // Offset within storage partition
const DEVICE_PROPERTIES_SIZE: usize = 256; // Max JSON size
const STORAGE_MAGIC: u32 = 0x44455650; // "DEVP" magic number

// Simple storage header
#[repr(C)]
struct StorageHeader {
    magic: u32,
    data_len: u32,
}

// Device properties structure for persistent storage
#[derive(Serialize, Deserialize, Debug, Clone)]
struct DeviceProperties {
    x: f32,
    y: f32,
    z: f32,
    // Future properties can be added here
}

impl Default for DeviceProperties {
    fn default() -> Self {
        Self {
            x: 1.0,
            y: 0.5,
            z: 2.0,
        }
    }
}

// Position update structure for MQTT messages
#[derive(Deserialize, Debug)]
struct PositionUpdate {
    x: f32,
    y: f32,
    z: f32,
}

use embassy_futures::yield_now;
use esp_radio::wifi::PowerSaveMode;

// Storage functions using proper storage partition access
fn load_device_properties() -> DeviceProperties {
    // SAFETY: We're stealing the FLASH peripheral for flash operations
    // This is safe because we're not using it elsewhere and flash operations are synchronous
    let flash = unsafe { esp_hal::peripherals::FLASH::steal() };
    let mut flash_storage = FlashStorage::new(flash);

    // Read partition table
    let mut pt_mem = [0u8; partitions::PARTITION_TABLE_MAX_LEN];
    let pt = match partitions::read_partition_table(&mut flash_storage, &mut pt_mem) {
        Ok(pt) => pt,
        Err(_) => {
            error!("Failed to read partition table");
            return DeviceProperties::default();
        }
    };

    // Find storage partition
    let storage_partition = match pt.find_partition(partitions::PartitionType::Data(
        partitions::DataPartitionSubType::Spiffs,
    )) {
        Ok(Some(partition)) => partition,
        _ => {
            error!("Storage partition not found");
            return DeviceProperties::default();
        }
    };

    let mut storage = storage_partition.as_embedded_storage(&mut flash_storage);
    let mut buffer = [0u8; core::mem::size_of::<StorageHeader>() + DEVICE_PROPERTIES_SIZE];

    // Try to read from storage partition at the fixed offset
    if storage.read(DEVICE_PROPERTIES_OFFSET, &mut buffer).is_ok() {
        // Check magic number
        let header = unsafe { &*(buffer.as_ptr() as *const StorageHeader) };
        if header.magic == STORAGE_MAGIC && header.data_len <= DEVICE_PROPERTIES_SIZE as u32 {
            let data_start = core::mem::size_of::<StorageHeader>();
            let data_end = data_start + header.data_len as usize;

            if data_end <= buffer.len() {
                match serde_json_core::from_slice::<DeviceProperties>(&buffer[data_start..data_end])
                {
                    Ok((props, _)) => {
                        info!("Loaded device properties from storage: {:?}", props);
                        return props;
                    }
                    Err(e) => {
                        error!("Failed to deserialize device properties: {:?}", e);
                    }
                }
            }
        }
    }

    info!("Using default device properties");
    DeviceProperties::default()
}

fn save_device_properties(props: &DeviceProperties) -> Result<(), &'static str> {
    // SAFETY: We're stealing the FLASH peripheral for flash operations
    // This is safe because we're not using it elsewhere and flash operations are synchronous
    let flash = unsafe { esp_hal::peripherals::FLASH::steal() };
    let mut flash_storage = FlashStorage::new(flash);

    // Read partition table
    let mut pt_mem = [0u8; partitions::PARTITION_TABLE_MAX_LEN];
    let pt = match partitions::read_partition_table(&mut flash_storage, &mut pt_mem) {
        Ok(pt) => pt,
        Err(_) => return Err("Failed to read partition table"),
    };

    // Find storage partition
    let storage_partition = match pt.find_partition(partitions::PartitionType::Data(
        partitions::DataPartitionSubType::Spiffs,
    )) {
        Ok(Some(partition)) => partition,
        _ => return Err("Storage partition not found"),
    };

    let mut storage = storage_partition.as_embedded_storage(&mut flash_storage);
    let mut json_buffer = [0u8; DEVICE_PROPERTIES_SIZE];

    // Serialize to JSON
    let json_len = match serde_json_core::to_slice(props, &mut json_buffer) {
        Ok(len) => len,
        Err(_) => return Err("Failed to serialize device properties"),
    };

    // Create storage buffer with header
    let mut storage_buffer = [0u8; core::mem::size_of::<StorageHeader>() + DEVICE_PROPERTIES_SIZE];
    let header = StorageHeader {
        magic: STORAGE_MAGIC,
        data_len: json_len as u32,
    };

    // Copy header
    let header_bytes = unsafe {
        core::slice::from_raw_parts(
            &header as *const _ as *const u8,
            core::mem::size_of::<StorageHeader>(),
        )
    };
    storage_buffer[..core::mem::size_of::<StorageHeader>()].copy_from_slice(header_bytes);

    // Copy JSON data
    let data_start = core::mem::size_of::<StorageHeader>();
    storage_buffer[data_start..data_start + json_len].copy_from_slice(&json_buffer[..json_len]);

    // Write to storage partition
    match storage.write(
        DEVICE_PROPERTIES_OFFSET,
        &storage_buffer[..data_start + json_len],
    ) {
        Ok(()) => {
            info!(
                "Device properties saved to storage successfully: {:?}",
                props
            );
            Ok(())
        }
        Err(_) => {
            error!("Failed to write device properties to flash");
            Err("Failed to write to flash")
        }
    }
}

#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    println!("Panic! {:?}", panic_info);
    loop {}
}

// fn heap_stats() {
//     let stats: HeapStats = esp_alloc::HEAP.stats();
//     // HeapStats implements the Display and defmt::Format traits, so you can pretty-print the heap stats.
//     println!("{}", stats);
//
// }

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    print!("System starting up...");
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    println!(" ok");

    esp_println::logger::init_logger_from_env();

    const MEMORY_SIZE: usize = 200 * 1024;
    print!("Initializing allocator with {} bytes...", MEMORY_SIZE);
    esp_alloc::heap_allocator!(size: MEMORY_SIZE);
    println!(" ok");

    let rng = Rng::new();

    // Initialize Wi-Fi controller with esp-rtos
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);
    let radio_init = &*mk_static!(
        esp_radio::Controller<'static>,
        esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller")
    );
    let (wifi_controller, interfaces) =
        esp_radio::wifi::new(radio_init, peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");
    let wifi_device = interfaces.sta;

    #[cfg(feature = "neopixel")]
    let led = {
        let led_pin = peripherals.GPIO8;
        let freq = esp_hal::time::Rate::from_mhz(80);
        let rmt = Rmt::new(peripherals.RMT, freq).unwrap();
        let rmt_buffer = smart_led_buffer!(1);
        let mut led = SmartLedsAdapter::new(rmt.channel0, led_pin, rmt_buffer);
        // Set the RGB color (e.g., Red)
        let color = RGB8 { r: 0, g: 0, b: 255 };

        // Write color data to NeoPixel with gamma correction and brightness adjustment
        led.write(brightness(gamma(core::iter::once(color)), LED_BRIGHTNESS))
            .unwrap();
        info!("LED ready");
        led
    };

    // heap_stats();

    let _server_ip: Ipv4Addr = SERVER_IP.parse().expect("Invalid SERVER_IP address");
    let config = embassy_net::Config::dhcpv4(Default::default());
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    // Create a unique device ID using a simple approach
    // In a real deployment, you could use hardware serial number or MAC address
    // For now, we'll use a random suffix based on the RNG
    let device_suffix = rng.random() as u32;
    let device_id =
        heapless::String::<64>::try_from(alloc::format!("esp32-c6-{:08x}", device_suffix).as_str())
            .unwrap();

    info!("Device ID: {}", device_id.as_str());

    // Log network configuration for debugging
    info!("Network Configuration:");
    info!("  SSID: {}", SSID);
    info!("  SERVER_IP: {}", SERVER_IP);
    info!("  MQTT Port: 1883");

    let (stack_local, runner) = embassy_net::new(
        wifi_device,
        config,
        mk_static!(StackResources<3>, StackResources::new()),
        seed,
    );
    // promote stack to 'static for tasks
    let stack = mk_static!(Stack<'static>, stack_local);
    let device_id_static = mk_static!(heapless::String<64>, device_id);

    #[cfg(feature = "neopixel")]
    {
        let led_static = mk_static!(SmartLedsAdapter<'static, 25>, led);

        spawner
            .spawn(hardware_task_runner(led_static, CHANNEL.receiver()))
            .unwrap();
    }

    spawner.spawn(connection(wifi_controller)).ok();
    spawner.spawn(net_task(runner)).ok();
    spawner.spawn(tick_task()).ok();
    // spawn a task to wait for network and launch MQTT
    spawner.spawn(mqtt_launcher(stack, device_id_static)).ok();

    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

#[embassy_executor::task]
async fn mqtt_launcher(stack: &'static Stack<'static>, device_id: &'static heapless::String<64>) {
    info!("Waiting for network connection...");
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
    // once we have an IP, connect socket and run MQTT
    loop {
        if let Some(config) = stack.config_v4() {
            info!("Got IP: {}", config.address);
            info!("Network config details:");
            info!("  IP: {}", config.address);
            info!("  Gateway: {:?}", config.gateway);
            info!("  DNS servers: {:?}", config.dns_servers);

            // static buffers
            let rx_buffer = mk_static!([u8; 4096], [0; 4096]);
            let tx_buffer = mk_static!([u8; 4096], [0; 4096]);
            let mut socket = TcpSocket::new(*stack, rx_buffer, tx_buffer);
            let server_ip: Ipv4Addr = SERVER_IP.parse().expect("Invalid SERVER_IP address");
            // Use standard MQTT port 1883 instead of 1884
            let remote_endpoint = (server_ip, 1883);

            info!(
                "Attempting to connect to MQTT broker at {}:{}",
                remote_endpoint.0, remote_endpoint.1
            );

            match socket.connect(remote_endpoint).await {
                Ok(()) => {
                    info!(
                        "Successfully connected to MQTT broker at {}:{}",
                        remote_endpoint.0, remote_endpoint.1
                    );
                    // hand off to mqtt_task
                    mqtt_task(socket, device_id).await;
                    break;
                }
                Err(e) => {
                    error!(
                        "Failed to connect to MQTT broker at {}:{} - Error: {:?}",
                        remote_endpoint.0, remote_endpoint.1, e
                    );
                    error!("This could be because:");
                    error!(
                        "1. MQTT broker is not running at {}:{}",
                        remote_endpoint.0, remote_endpoint.1
                    );
                    error!("2. Network connectivity issues");
                    error!("3. Firewall blocking the connection");
                    error!("4. Wrong IP address or port");
                    error!("Retrying connection in 5 seconds...");
                    Timer::after(Duration::from_millis(5000)).await;
                    continue;
                }
            }
        }
        Timer::after(Duration::from_millis(500)).await;
    }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.capabilities());

    // Disable PowerSaveMode to keep radio fully operational
    println!("Disabling PowerSaveMode to avoid delay when receiving data.");
    controller.set_power_saving(PowerSaveMode::None).unwrap();

    loop {
        match esp_radio::wifi::sta_state() {
            WifiStaState::Connected => {
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await;
            }
            _ => {}
        }

        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = ModeConfig::Client(
                WifiClientConfig::default()
                    .with_ssid(SSID.into())
                    .with_password(PASSWORD.into()),
            );
            controller.set_config(&client_config).unwrap();
            println!("Starting wifi");
            controller.start_async().await.unwrap();
            println!("Wifi started!");
        }

        println!("About to connect...");
        match controller.connect_async().await {
            Ok(_) => println!("Wifi connected!"),
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await;
            }
        }
    }
}

#[embassy_executor::task]
async fn tick_task() {
    loop {
        // Check the busy state and print it
        let busy = IS_BUSY.load(Ordering::Relaxed);
        if busy {
            println!("Tick... BUSY processing packets");
        } else {
            println!("Tick... IDLE");
        }

        yield_now().await; // Yield to allow other tasks to run
        Timer::after(Duration::from_secs(1)).await;
    }
}

#[derive(Debug)]
#[allow(dead_code)]
enum HardwareEvent {
    ToggleLed,
    TurnOnLed,
    TurnOffLed, // Future events can be added here (e.g., ButtonPressed, DisplayUpdate, etc.)
}

#[cfg(feature = "neopixel")]
#[embassy_executor::task]
async fn hardware_task_runner(
    led: &'static mut SmartLedsAdapter<'static, 25>,
    receiver: embassy_sync::channel::Receiver<'static, CriticalSectionRawMutex, HardwareEvent, 1>,
) {
    let mut toggle_state: u8 = 0;

    loop {
        let event = receiver.receive().await;

        match event {
            HardwareEvent::ToggleLed => {
                println!("Toggle led");
                toggle_state = (toggle_state + 1) % 3;
                let color = match toggle_state {
                    0 => RGB8 { r: 255, g: 0, b: 0 }, // Red
                    1 => RGB8 { r: 0, g: 255, b: 0 }, // Green
                    _ => RGB8 { r: 0, g: 0, b: 0 },   // Off
                };

                led.write(brightness(gamma(core::iter::once(color)), LED_BRIGHTNESS))
                    .unwrap();
            }
            HardwareEvent::TurnOffLed => {
                println!("Turn off led");
                toggle_state = 0;
                let color = RGB8 { r: 0, g: 0, b: 0 };

                led.write(brightness(gamma(core::iter::once(color)), LED_BRIGHTNESS))
                    .unwrap();
            }
            HardwareEvent::TurnOnLed => {
                println!("Turn on led");
                toggle_state = 0;
                let color = RGB8 {
                    r: 255,
                    g: 255,
                    b: 0,
                };

                led.write(brightness(gamma(core::iter::once(color)), LED_BRIGHTNESS))
                    .unwrap();
            }
        }
        yield_now().await;
    }
}

async fn mqtt_task(socket: TcpSocket<'static>, device_id: &'static heapless::String<64>) {
    // Load device properties from storage
    let mut device_props = load_device_properties();
    info!("Loaded device properties: {:?}", device_props);

    // allocate buffers for the client
    let mut recv_buffer = [0u8; 512];
    let mut write_buffer = [0u8; 512];
    let recv_buffer_len = recv_buffer.len();
    let write_buffer_len = write_buffer.len();
    // configure the MQTT client
    let mut config = ClientConfig::new(
        rust_mqtt::client::client_config::MqttVersion::MQTTv5,
        CountingRng(20000),
    );
    config.add_max_subscribe_qos(QualityOfService::QoS1);
    config.add_client_id(device_id.as_str());
    config.max_packet_size = 200;

    info!("MQTT connecting to broker at {}:1884", SERVER_IP);

    // create the MQTT client
    let mut client = MqttClient::<_, 5, _>::new(
        socket,
        &mut write_buffer,
        write_buffer_len,
        &mut recv_buffer,
        recv_buffer_len,
        config,
    );
    loop {
        // connect to broker with retry on failure
        if let Err(err) = client.connect_to_broker().await {
            error!("MQTT connect error: {:?}", err);
            Timer::after(Duration::from_secs(5)).await;
            continue;
        }

        // subscribe to lamp control topic
        let light_topic = alloc::format!("home/{}/light", device_id.as_str());
        if let Err(err) = client.subscribe_to_topic(&light_topic).await {
            error!("MQTT subscribe to light topic error: {:?}", err);
            Timer::after(Duration::from_secs(5)).await;
            continue;
        }
        info!("Subscribed to light topic: {}", light_topic);

        // subscribe to position/set topic
        let position_topic = alloc::format!("home/{}/position/set", device_id.as_str());
        if let Err(err) = client.subscribe_to_topic(&position_topic).await {
            error!("MQTT subscribe to position topic error: {:?}", err);
            Timer::after(Duration::from_secs(5)).await;
            continue;
        }
        info!("Subscribed to position topic: {}", position_topic);

        break;
    }

    // Send device announcement message with persisted coordinates
    let announcement = alloc::format!(
        r#"{{"device_id":"{}","device_type":"lamp","state":"online","location":{{"x":{},"y":{},"z":{}}}}}"#,
        device_id.as_str(),
        device_props.x,
        device_props.y,
        device_props.z
    );
    if let Err(err) = client
        .send_message(
            "devices/announce",
            announcement.as_bytes(),
            QualityOfService::QoS1,
            false,
        )
        .await
    {
        error!("Failed to send device announcement: {:?}", err);
    } else {
        info!(
            "Device announcement sent successfully with position: x={}, y={}, z={}",
            device_props.x, device_props.y, device_props.z
        );
    }

    // process incoming messages
    loop {
        if let Ok((topic, payload)) = client.receive_message().await {
            let topic_str = alloc::string::String::from_utf8_lossy(topic.as_bytes()).to_string();
            let payload_str = alloc::string::String::from_utf8_lossy(payload).to_string();

            info!(
                "Received MQTT message on topic '{}': {}",
                topic_str, payload_str
            );

            // Check if it's a light control message
            if topic_str.ends_with("/light") {
                match payload_str.as_str() {
                    "ON" => {
                        let _ = CHANNEL.sender().try_send(HardwareEvent::TurnOnLed);
                    }
                    "OFF" => {
                        let _ = CHANNEL.sender().try_send(HardwareEvent::TurnOffLed);
                    }
                    _ => {}
                }
            }
            // Check if it's a position update message
            else if topic_str.ends_with("/position/set") {
                info!("Processing position update: {}", payload_str);

                // Parse JSON position update
                let mut json_buffer = [0u8; 256];
                if payload_str.len() <= json_buffer.len() {
                    json_buffer[..payload_str.len()].copy_from_slice(payload_str.as_bytes());

                    match serde_json_core::from_slice::<PositionUpdate>(
                        &json_buffer[..payload_str.len()],
                    ) {
                        Ok((position_update, _)) => {
                            info!("Parsed position update: {:?}", position_update);

                            // Update device properties
                            device_props.x = position_update.x;
                            device_props.y = position_update.y;
                            device_props.z = position_update.z;

                            // Save to persistent storage
                            match save_device_properties(&device_props) {
                                Ok(()) => {
                                    info!(
                                        "Position updated and saved: x={}, y={}, z={}",
                                        device_props.x, device_props.y, device_props.z
                                    );
                                }
                                Err(e) => {
                                    error!("Failed to save position update: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse position update JSON: {:?}", e);
                        }
                    }
                } else {
                    error!(
                        "Position update payload too large: {} bytes",
                        payload_str.len()
                    );
                }
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}
