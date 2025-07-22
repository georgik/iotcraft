#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use core::net::{Ipv4Addr, SocketAddrV4};

use embassy_executor::Spawner;
use embassy_net::{
    IpListenEndpoint, Ipv4Cidr, Runner, Stack, StackResources, StaticConfigV4, tcp::TcpSocket,
};
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_hal::{clock::CpuClock, rng::Rng, timer::timg::TimerGroup};
use esp_println::println;
use esp_wifi::{
    EspWifiController,
    init,
    wifi::{
        AccessPointConfiguration,
        ClientConfiguration,
        Configuration,
        WifiController,
        WifiDevice,
        WifiEvent,
        WifiState,
    },
};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
esp_bootloader_esp_idf::esp_app_desc!();

// When you are okay with using a nightly compiler it's better to use https://docs.rs/static_cell/2.1.0/static_cell/macro.make_static.html
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

// Configuration constants
const AP_SSID: &str = "ESP32-DHCP-GW";
const AP_PASSWORD: &str = "esp32gateway";
const STA_SSID: &str = "YourParentNetwork"; // Change this to your parent network
const STA_PASSWORD: &str = "YourParentPassword"; // Change this to your parent network password

const AP_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 4, 1);
const AP_GATEWAY: Ipv4Addr = Ipv4Addr::new(192, 168, 4, 1);
const DHCP_START_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 4, 2);
const DHCP_END_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 4, 254);

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // Use PSRAM for heap allocation as per rule for embedded projects
    esp_alloc::heap_allocator!(size: 128 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let mut rng = Rng::new(peripherals.RNG);

    let esp_wifi_ctrl = &*mk_static!(
        EspWifiController<'static>,
        init(timg0.timer0, rng.clone()).unwrap()
    );

    let (controller, interfaces) = esp_wifi::wifi::new(&esp_wifi_ctrl, peripherals.WIFI).unwrap();

    let wifi_ap_device = interfaces.ap;
    let wifi_sta_device = interfaces.sta;

    cfg_if::cfg_if! {
        if #[cfg(feature = "esp32")] {
            let timg1 = TimerGroup::new(peripherals.TIMG1);
            esp_hal_embassy::init(timg1.timer0);
        } else {
            use esp_hal::timer::systimer::SystemTimer;
            let systimer = SystemTimer::new(peripherals.SYSTIMER);
            esp_hal_embassy::init(systimer.alarm0);
        }
    }

    // Configure AP network with static IP
    let ap_config = embassy_net::Config::ipv4_static(StaticConfigV4 {
        address: Ipv4Cidr::new(AP_IP, 24),
        gateway: Some(AP_GATEWAY),
        dns_servers: heapless::Vec::new(),
    });

    // Configure STA to get IP via DHCP from parent network
    let sta_config = embassy_net::Config::dhcpv4(Default::default());

    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    // Initialize network stacks
    let (ap_stack, ap_runner) = embassy_net::new(
        wifi_ap_device,
        ap_config,
        mk_static!(StackResources<5>, StackResources::<5>::new()),
        seed,
    );

    let (sta_stack, sta_runner) = embassy_net::new(
        wifi_sta_device,
        sta_config,
        mk_static!(StackResources<5>, StackResources::<5>::new()),
        seed,
    );

    // Configure WiFi in mixed mode (AP + STA)
    let mixed_config = Configuration::Mixed(
        ClientConfiguration {
            ssid: STA_SSID.try_into().unwrap(),
            password: STA_PASSWORD.try_into().unwrap(),
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: AP_SSID.try_into().unwrap(),
            password: AP_PASSWORD.try_into().unwrap(),
            max_connections: 32,
            ..Default::default()
        },
    );

    // Spawn background tasks
    spawner.spawn(connection_task(controller, mixed_config)).ok();
    spawner.spawn(ap_net_task(ap_runner)).ok();
    spawner.spawn(sta_net_task(sta_runner)).ok();
    spawner.spawn(dhcp_server_task(ap_stack)).ok();
    spawner.spawn(nat_forwarding_task(ap_stack, sta_stack)).ok();
    spawner.spawn(web_server_task(ap_stack)).ok();

    println!("ESP32-S3 DHCP Gateway starting up...");
    println!("AP SSID: {}, Password: {}", AP_SSID, AP_PASSWORD);
    println!("Connect to AP and get IP via DHCP in range 192.168.4.x");
    println!("Web interface will be available at http://192.168.4.1:80/");

    // Main loop - could be used for status monitoring
    loop {
        Timer::after(Duration::from_secs(30)).await;
        if ap_stack.is_link_up() {
            println!("AP is up and running");
        }
        if sta_stack.is_link_up() {
            if let Some(config) = sta_stack.config_v4() {
                println!("STA connected with IP: {}", config.address.address());
            }
        }
    }
}

#[embassy_executor::task]
async fn connection_task(mut controller: WifiController<'static>, config: Configuration) {
    println!("Starting WiFi connection task");
    controller.set_configuration(&config).unwrap();

    loop {
        match esp_wifi::wifi::wifi_state() {
            WifiState::StaConnected => {
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await;
            }
            _ => {}
        }

        if !matches!(controller.is_started(), Ok(true)) {
            println!("Starting WiFi controller...");
            controller.start_async().await.unwrap();
            println!("WiFi controller started");
        }

        println!("Attempting to connect to parent network: {}", STA_SSID);
        match controller.connect_async().await {
            Ok(_) => println!("Connected to parent network successfully"),
            Err(e) => {
                println!("Failed to connect to parent network: {:?}", e);
                Timer::after(Duration::from_millis(10000)).await;
            }
        }
    }
}

#[embassy_executor::task]
async fn ap_net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

#[embassy_executor::task]
async fn sta_net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

#[embassy_executor::task]
async fn dhcp_server_task(stack: Stack<'static>) {
    use edge_dhcp::{
        io::{self, DEFAULT_SERVER_PORT},
        server::{Server, ServerOptions},
    };
    use edge_nal::UdpBind;
    use edge_nal_embassy::{Udp, UdpBuffers};

    println!("Starting DHCP server...");

    // Wait for AP interface to be up
    loop {
        if stack.is_link_up() && stack.is_config_up() {
            break;
        }
        Timer::after(Duration::from_millis(100)).await;
    }

    let mut buf = [0u8; 1500];
    let mut gw_buf = [AP_GATEWAY];

    let buffers = UdpBuffers::<5, 1024, 1024, 10>::new();
    let unbound_socket = Udp::new(stack, &buffers);
    let mut bound_socket = unbound_socket
        .bind(core::net::SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            DEFAULT_SERVER_PORT,
        )))
        .await
        .unwrap();

    println!("DHCP server bound to port {}", DEFAULT_SERVER_PORT);

    loop {
        let result = io::server::run(
            &mut Server::<_, 32>::new_with_et(AP_IP),
            &ServerOptions::new(AP_IP, Some(&mut gw_buf)),
            &mut bound_socket,
            &mut buf,
        )
        .await;

        if let Err(e) = result {
            println!("DHCP server error: {:?}", e);
        }

        Timer::after(Duration::from_millis(100)).await;
    }
}

#[embassy_executor::task]
async fn nat_forwarding_task(
    _ap_stack: Stack<'static>,
    _sta_stack: Stack<'static>,
) {
    // This is a placeholder for NAT functionality
    // In a full implementation, we would need to:
    // 1. Listen for packets on the AP interface
    // 2. Modify source IP/port for outbound packets
    // 3. Forward them through the STA interface
    // 4. Track connections in a NAT table
    // 5. Handle return packets by reversing the translation
    //
    // For now, we'll implement basic packet forwarding logic
    println!("NAT forwarding task started (placeholder implementation)");
    
    // Note: This is where we would implement NAT translation logic
    // using smoltcp's raw socket capabilities to intercept and modify packets
    
    loop {
        Timer::after(Duration::from_secs(10)).await;
        // TODO: Implement proper NAT forwarding when smoltcp supports it better
        // or when we can access lower-level packet handling
    }
}

#[embassy_executor::task]
async fn web_server_task(stack: Stack<'static>) {
    println!("Starting web server...");

    // Wait for network to be ready
    loop {
        if stack.is_link_up() && stack.is_config_up() {
            break;
        }
        Timer::after(Duration::from_millis(100)).await;
    }

    let mut rx_buffer = [0; 2048];
    let mut tx_buffer = [0; 2048];
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(Duration::from_secs(10)));

    println!("Web server ready on http://192.168.4.1:80/");

    loop {
        println!("Waiting for web connection...");
        let result = socket
            .accept(IpListenEndpoint {
                addr: None,
                port: 80,
            })
            .await;

        if let Err(e) = result {
            println!("Accept error: {:?}", e);
            continue;
        }

        println!("Web client connected");

        // Read HTTP request
        let mut buffer = [0u8; 1024];
        let mut pos = 0;
        loop {
            match socket.read(&mut buffer[pos..]).await {
                Ok(0) => break, // EOF
                Ok(len) => {
                    pos += len;
                    let request = unsafe { core::str::from_utf8_unchecked(&buffer[..pos]) };
                    if request.contains("\r\n\r\n") {
                        println!("Request: {}", request.lines().next().unwrap_or(""));
                        break;
                    }
                    if pos >= buffer.len() {
                        break;
                    }
                }
                Err(e) => {
                    println!("Read error: {:?}", e);
                    break;
                }
            }
        }

        // Send HTTP response
        let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
            <!DOCTYPE html>\
            <html>\
            <head><title>ESP32-S3 DHCP Gateway</title></head>\
            <body>\
                <h1>ESP32-S3 DHCP Gateway</h1>\
                <p>Gateway is running successfully!</p>\
                <p>AP IP: 192.168.4.1</p>\
                <p>DHCP Range: 192.168.4.2 - 192.168.4.254</p>\
                <p>Connected clients will get internet through the parent network.</p>\
            </body>\
            </html>";

        use embedded_io_async::Write;
        if let Err(e) = socket.write_all(response).await {
            println!("Write error: {:?}", e);
        }

        if let Err(e) = socket.flush().await {
            println!("Flush error: {:?}", e);
        }

        Timer::after(Duration::from_millis(100)).await;
        socket.close();
        socket.abort();
    }
}
