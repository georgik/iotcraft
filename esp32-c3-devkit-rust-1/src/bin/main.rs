#![no_std]
#![no_main]
extern crate alloc;
use esp_wifi::wifi::WifiDevice;
use esp_hal::gpio::Level;
use esp_hal::gpio::Output;
use esp_hal::{delay, Blocking};
use esp_hal::dma::DmaPriority;
use esp_hal::dma::Owner::Dma;
use esp_hal::spi::master::Spi;
use esp_hal::timer::systimer::SystemTimer;
use heapless::String;
use core::net::Ipv4Addr;
use log::{info, error};
use embedded_hal::delay::DelayNs;
use alloc::vec::Vec;
use crate::alloc::string::ToString;

use embassy_executor::Spawner;
use embassy_net::{tcp::TcpSocket, Runner, StackResources, Stack};
use embassy_time::{Duration, Instant, Timer};
use embedded_io_async::Write;
use esp_alloc as _;
use esp_alloc::HeapStats;
// use esp_backtrace as _;
use esp_hal::{clock::CpuClock, rng::Rng, timer::timg::TimerGroup, delay::Delay,};
use esp_println::{print, println};
use esp_wifi::{
    init,
    wifi::{ClientConfiguration, Configuration, WifiController, WifiEvent, WifiState},
    EspWifiController,
};

use esp_hal::rmt::Rmt;
use esp_hal_smartled::{smart_led_buffer, SmartLedsAdapter};

use smart_leds::{brightness, gamma, hsv::{hsv2rgb, Hsv}, SmartLedsWrite, RGB8};

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use esp_hal::rmt::TxChannel;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
// use rumqttc::{MqttOptions, Client, QoS};
// use rumqttc::{Event, Outgoing};

use rust_mqtt::{
    client::client::MqttClient,
    client::client_config::ClientConfig,
    packet::v5::publish_packet::QualityOfService,
    utils::rng_generator::CountingRng,
};

use esp_hal::i2c::master::I2c;
use shtcx::{shtc3, PowerMode};

use heapless::Vec as HVec;
#[derive(Clone)]
struct OutgoingMsg {
    topic: &'static str,
    payload: HVec<u8, 64>,
}

static PUBLISH_CHANNEL: Channel<CriticalSectionRawMutex, OutgoingMsg, 4> = Channel::new();

// Define a static channel with a capacity of 1 for `HardwareEvent`s.
static CHANNEL: Channel<CriticalSectionRawMutex, HardwareEvent, 1> = Channel::new();
use core::sync::atomic::{AtomicBool, Ordering};
static IS_BUSY: AtomicBool = AtomicBool::new(false);

macro_rules! mk_static {
    ($t:ty, $val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        STATIC_CELL.init($val)
    }};
}

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");
const SERVER_IP: &str = env!("SERVER_IP");


use embassy_futures::yield_now;
use esp_wifi::config::PowerSaveMode;

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


#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    print!("System starting up...");
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    println!(" ok");

    esp_println::logger::init_logger_from_env();

    const memory_size: usize = 200 * 1024;
    print!("Initializing allocator with {} bytes...", memory_size);
    esp_alloc::heap_allocator!(size: memory_size);
    println!(" ok");


    let mut rng = Rng::new(peripherals.RNG);

    // Initialize Wi-Fi controller
    let timer1 = TimerGroup::new(peripherals.TIMG0);
    let wifi_init = &*mk_static!(
        EspWifiController<'static>,
        init(timer1.timer0, rng.clone(), peripherals.RADIO_CLK).unwrap()
    );
    // let wifi_init = esp_wifi::init(timer1.timer0, rng, peripherals.RADIO_CLK)
    //     .expect("Failed to initialize WIFI/BLE controller");
    let (mut wifi_controller, interfaces) = esp_wifi::wifi::new(&wifi_init, peripherals.WIFI)
        .expect("Failed to initialize WIFI controller");
    let wifi_device = interfaces.sta;


    let led_pin = peripherals.GPIO2;
    let freq = esp_hal::time::Rate::from_mhz(80);
    let rmt = Rmt::new(peripherals.RMT, freq).unwrap();
    let rmt_buffer = smart_led_buffer!(1);
    let mut led = SmartLedsAdapter::new(rmt.channel0, led_pin, rmt_buffer);
    // Set the RGB color (e.g., Red)
    let color = RGB8 { r: 0, g: 0, b: 255 };

    // Write color data to NeoPixel with gamma correction and brightness adjustment
    led.write(brightness(gamma(core::iter::once(color)), 10))
        .unwrap();


    info!("SPI ready");


    // heap_stats();

    let systimer = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(systimer.alarm0);

    let server_ip: Ipv4Addr = SERVER_IP.parse().expect("Invalid SERVER_IP address");
    let config = embassy_net::Config::dhcpv4(Default::default());
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let (stack_local, runner) = embassy_net::new(
        wifi_device,
        config,
        mk_static!(StackResources<3>, StackResources::new()),
        seed,
    );
    // promote stack to 'static for tasks
    let stack = mk_static!(Stack<'static>, stack_local);

    // Initialize I²C for the SHTC3 temperature sensor (SDA=GPIO10, SCL=GPIO8)
    let i2c_bus: I2cMasterBus = I2c::new(peripherals.I2C0, esp_hal::i2c::master::Config::default())
        .unwrap()
        .with_sda(peripherals.GPIO10)
        .with_scl(peripherals.GPIO8);

    // Spawn a task to read temperature every minute and publish via MQTT
    spawner.spawn(sensor_task(i2c_bus)).unwrap();

    spawner
        .spawn(hardware_task_runner(led, CHANNEL.receiver()))
        .unwrap();

    spawner.spawn(connection(wifi_controller)).ok();
    spawner.spawn(net_task(runner)).ok();
    spawner.spawn(tick_task()).ok();
    // spawn a task to wait for network and launch MQTT
    // spawn the launcher task which will create the socket and then call mqtt_task
    spawner.spawn(mqtt_launcher(stack)).ok();

}


#[embassy_executor::task]
async fn mqtt_launcher(stack: &'static Stack<'static>) {
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
            // static buffers
            let rx_buffer = mk_static!([u8; 4096], [0; 4096]);
            let tx_buffer = mk_static!([u8; 4096], [0; 4096]);
            let mut socket = TcpSocket::new(*stack, rx_buffer, tx_buffer);
            let server_ip: Ipv4Addr = SERVER_IP.parse().expect("Invalid SERVER_IP address");
            let remote_endpoint = (server_ip, 1884);
            socket.connect(remote_endpoint).await.unwrap();
            info!("Connected to MQTT Socket at {}", remote_endpoint.0);
            // hand off to mqtt_task with publish-channel
            mqtt_task(socket, PUBLISH_CHANNEL.receiver()).await;
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
}



#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.capabilities());

    // https://docs.esp-rs.org/esp-hal/esp-wifi/0.12.0/esp32c6/esp_wifi/#wifi-performance-considerations
    println!("Disabling PowerSaveMode to avoid delay when receiving data.");
    controller.set_power_saving(PowerSaveMode::None).unwrap();

    loop {
        match esp_wifi::wifi::wifi_state() {
            WifiState::StaConnected => {
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await;
            }
            _ => {}
        }

        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASSWORD.try_into().unwrap(),
                auth_method: if !PASSWORD.is_empty() {
                    esp_wifi::wifi::AuthMethod::WPA2Personal
                } else {
                    // No password. E.g., in case of Wokwi simulator
                    esp_wifi::wifi::AuthMethod::None
                },
                ..Default::default()
            });

            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi");
            controller.start_async().await.unwrap();
            println!("Wifi started!");
        }

        println!("About to connect to {}", SSID);
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
enum HardwareEvent {
    ToggleLed,
    TurnOnLed,
    TurnOffLed
    // Future events can be added here (e.g., ButtonPressed, DisplayUpdate, etc.)
}


#[embassy_executor::task]
async fn hardware_task_runner(
    mut led: SmartLedsAdapter<esp_hal::rmt::Channel<esp_hal::Blocking, 0>, 25>,
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

                led.write(brightness(gamma(core::iter::once(color)), 10))
                    .unwrap();
            }
            HardwareEvent::TurnOffLed => {
                println!("Turn off led");
                toggle_state = 0;
                let color = RGB8 { r: 0, g: 0, b: 0 };

                led.write(brightness(gamma(core::iter::once(color)), 10))
                    .unwrap();
            }
            HardwareEvent::TurnOnLed => {
                println!("Turn on led");
                toggle_state = 0;
                let color = RGB8 { r: 255, g: 255, b: 0 };

                led.write(brightness(gamma(core::iter::once(color)), 10))
                    .unwrap();
            }

        }
        yield_now().await;
    }
}

async fn mqtt_task(mut socket: TcpSocket<'static>, mut pub_rx: embassy_sync::channel::Receiver<'static, CriticalSectionRawMutex, OutgoingMsg, 4>) {
    // allocate buffers for the client
    let mut recv_buffer = [0u8; 80];
    let mut write_buffer = [0u8; 80];
    let recv_buffer_len = recv_buffer.len();
    let write_buffer_len = write_buffer.len();
    // configure the MQTT client
    let mut config = ClientConfig::new(
        rust_mqtt::client::client_config::MqttVersion::MQTTv5,
        CountingRng(20000),
    );
    config.add_max_subscribe_qos(QualityOfService::QoS1);
    config.add_client_id("esp32-client");
    config.max_packet_size = 100;

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
        // subscribe to lamp topic with retry on failure
        if let Err(err) = client.subscribe_to_topic("home/cube/light").await {
            error!("MQTT subscribe error: {:?}", err);
            Timer::after(Duration::from_secs(5)).await;
            continue;
        }
        break;
    }
    // process outgoing publishes
    loop {
        let msg = pub_rx.receive().await;
        // send each message through existing client
        if let Err(err) = client.send_message(
            msg.topic,
            &msg.payload,
            QualityOfService::QoS0,
            false,
        ).await {
            error!("MQTT publish error: {:?}", err);
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await;
}

type I2cMasterBus = esp_hal::i2c::master::I2c<'static, Blocking>;
type I2cMasterBusError = esp_hal::i2c::master::Error;

#[embassy_executor::task]
async fn sensor_task(mut i2c: I2cMasterBus) {
    // Create the SHTC3 driver
    let mut sensor =  shtc3(i2c);
    let mut delay = esp_hal::delay::Delay::new();

    loop {
        // Measure temperature (and humidity, if needed)
        let measurement = sensor.measure(PowerMode::NormalMode, &mut delay).unwrap();
        let temperature = measurement.temperature;
        let celsius = temperature.as_degrees_celsius() - 10.0;

        // Format payload
        let payload = alloc::format!("{:?}", celsius);
        info!("Sensor measurement: {} C", payload);

        // Publish over MQTT asynchronously through channel
        let mut buf = HVec::<u8, 64>::new();
        buf.extend_from_slice(payload.as_bytes()).unwrap();
        let _ = PUBLISH_CHANNEL.sender().try_send(OutgoingMsg {
            topic: "home/sensor/temperature",
            payload: buf,
        });

        // wait for a minute
        Timer::after(Duration::from_secs(60)).await;
    }
}

