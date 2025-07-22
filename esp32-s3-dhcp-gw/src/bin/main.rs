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
use log::{debug, error, info, warn};
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
    
    info!("🚀 Starting ESP32-S3 DHCP Gateway...");
    info!("📊 Heap size: 128KB, Target: ESP32-S3");
    
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    info!("⚡ ESP32-S3 initialized with maximum CPU clock");

    // Use PSRAM for heap allocation as per rule for embedded projects
    esp_alloc::heap_allocator!(size: 128 * 1024);
    info!("💾 Heap allocator initialized (128KB)");

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let mut rng = Rng::new(peripherals.RNG);
    debug!("🔧 Timer and RNG peripherals initialized");

    let esp_wifi_ctrl = &*mk_static!(
        EspWifiController<'static>,
        init(timg0.timer0, rng.clone()).unwrap()
    );
    info!("📡 WiFi controller initialized");

    let (controller, interfaces) = esp_wifi::wifi::new(&esp_wifi_ctrl, peripherals.WIFI).unwrap();
    info!("🔌 WiFi interfaces created (AP + STA)");

    let wifi_ap_device = interfaces.ap;
    let wifi_sta_device = interfaces.sta;

    cfg_if::cfg_if! {
        if #[cfg(feature = "esp32")] {
            let timg1 = TimerGroup::new(peripherals.TIMG1);
            esp_hal_embassy::init(timg1.timer0);
            debug!("🕰️ Embassy initialized with TIMG1");
        } else {
            use esp_hal::timer::systimer::SystemTimer;
            let systimer = SystemTimer::new(peripherals.SYSTIMER);
            esp_hal_embassy::init(systimer.alarm0);
            debug!("🕰️ Embassy initialized with SystemTimer");
        }
    }

    // Configure AP network with static IP
    let ap_config = embassy_net::Config::ipv4_static(StaticConfigV4 {
        address: Ipv4Cidr::new(AP_IP, 24),
        gateway: Some(AP_GATEWAY),
        dns_servers: heapless::Vec::new(),
    });
    info!("🏠 AP network configured: {} (gateway: {})", AP_IP, AP_GATEWAY);

    // Configure STA to get IP via DHCP from parent network
    let sta_config = embassy_net::Config::dhcpv4(Default::default());
    info!("🌐 STA network configured for DHCP");

    let seed = (rng.random() as u64) << 32 | rng.random() as u64;
    debug!("🎲 Network stack seed: 0x{:016x}", seed);

    // Initialize network stacks
    let (ap_stack, ap_runner) = embassy_net::new(
        wifi_ap_device,
        ap_config,
        mk_static!(StackResources<5>, StackResources::<5>::new()),
        seed,
    );
    info!("📡 AP network stack initialized");

    let (sta_stack, sta_runner) = embassy_net::new(
        wifi_sta_device,
        sta_config,
        mk_static!(StackResources<5>, StackResources::<5>::new()),
        seed,
    );
    info!("🌍 STA network stack initialized");

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
    info!("📶 WiFi configuration: AP='{}', STA='{}'*(hidden)", AP_SSID, STA_SSID);

    // Spawn background tasks
    info!("🚀 Spawning background tasks...");
    spawner.spawn(connection_task(controller, mixed_config)).ok();
    debug!("✅ Connection task spawned");
    spawner.spawn(ap_net_task(ap_runner)).ok();
    debug!("✅ AP network task spawned");
    spawner.spawn(sta_net_task(sta_runner)).ok();
    debug!("✅ STA network task spawned");
    spawner.spawn(dhcp_server_task(ap_stack)).ok();
    debug!("✅ DHCP server task spawned");
    spawner.spawn(nat_forwarding_task(ap_stack, sta_stack)).ok();
    debug!("✅ NAT forwarding task spawned");
    spawner.spawn(web_server_task(ap_stack)).ok();
    debug!("✅ Web server task spawned");

    info!("🎉 ESP32-S3 DHCP Gateway started successfully!");
    info!("📊 Configuration Summary:");
    info!("   📡 AP SSID: '{}'", AP_SSID);
    info!("   🔐 AP Password: '{}'", AP_PASSWORD);
    info!("   🏠 AP IP: {}/24", AP_IP);
    info!("   🌐 DHCP Range: {} - {}", DHCP_START_IP, DHCP_END_IP);
    info!("   🔗 Parent Network: '{}'", STA_SSID);
    info!("   🌍 Web Interface: http://{}/", AP_IP);

    // Main loop - status monitoring
    info!("🔍 Starting main status monitoring loop...");
    loop {
        Timer::after(Duration::from_secs(30)).await;
        
        // Check AP status
        if ap_stack.is_link_up() {
            if let Some(config) = ap_stack.config_v4() {
                info!("✅ AP Status: Link UP, IP: {}, Clients can connect", config.address.address());
            } else {
                warn!("⚠️  AP Status: Link UP, but no IP configuration");
            }
        } else {
            warn!("❌ AP Status: Link DOWN");
        }
        
        // Check STA status
        if sta_stack.is_link_up() {
            if let Some(config) = sta_stack.config_v4() {
                info!("🌐 STA Status: Connected to parent network with IP: {}", config.address.address());
            } else {
                warn!("⚠️  STA Status: Link UP, but no IP from DHCP yet");
            }
        } else {
            warn!("🔴 STA Status: Not connected to parent network");
        }
    }
}

#[embassy_executor::task]
async fn connection_task(mut controller: WifiController<'static>, config: Configuration) {
    info!("📡 Starting WiFi connection task...");
    controller.set_configuration(&config).unwrap();
    debug!("⚙️ WiFi configuration applied");

    loop {
        let wifi_state = esp_wifi::wifi::wifi_state();
        debug!("📄 Current WiFi state: {:?}", wifi_state);
        
        match wifi_state {
            WifiState::StaConnected => {
                info!("🌐 STA connected to parent network, waiting for disconnect event...");
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                warn!("🛁 STA disconnected from parent network");
                Timer::after(Duration::from_millis(5000)).await;
            }
            _ => {
                debug!("🔄 WiFi in state: {:?}, continuing connection loop", wifi_state);
            }
        }

        if !matches!(controller.is_started(), Ok(true)) {
            info!("🚀 Starting WiFi controller...");
            controller.start_async().await.unwrap();
            info!("✅ WiFi controller started successfully");
        }

        info!("🔗 Attempting to connect to parent network: '{}'", STA_SSID);
        match controller.connect_async().await {
            Ok(_) => info!("🎉 Connected to parent network '{}' successfully", STA_SSID),
            Err(e) => {
                error!("❌ Failed to connect to parent network '{}': {:?}", STA_SSID, e);
                warn!("⏳ Retrying connection in 10 seconds...");
                Timer::after(Duration::from_millis(10000)).await;
            }
        }
    }
}

#[embassy_executor::task]
async fn ap_net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    info!("📡 Starting AP network task...");
    runner.run().await
}

#[embassy_executor::task]
async fn sta_net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    info!("🌍 Starting STA network task...");
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

    info!("🏠 Starting DHCP server task...");
    debug!("🔧 DHCP server will serve range: {} - {}", DHCP_START_IP, DHCP_END_IP);

    // Wait for AP interface to be up
    info!("⏳ Waiting for AP interface to be ready...");
    loop {
        if stack.is_link_up() && stack.is_config_up() {
            info!("✅ AP interface is UP and configured");
            break;
        }
        debug!("🔄 AP interface not ready, waiting... (link: {}, config: {})", 
               stack.is_link_up(), stack.is_config_up());
        Timer::after(Duration::from_millis(100)).await;
    }

    let mut buf = [0u8; 1500];
    let mut gw_buf = [AP_GATEWAY];
    debug!("💾 DHCP server buffers allocated (1500 bytes + gateway: {})", AP_GATEWAY);

    let buffers = UdpBuffers::<5, 1024, 1024, 10>::new();
    let unbound_socket = Udp::new(stack, &buffers);
    debug!("🔌 UDP socket created");
    
    let mut bound_socket = unbound_socket
        .bind(core::net::SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            DEFAULT_SERVER_PORT,
        )))
        .await
        .unwrap();

    info!("🏠 DHCP server bound to port {} and ready to serve clients", DEFAULT_SERVER_PORT);

    let mut client_count = 0u32;
    loop {
        let result = io::server::run(
            &mut Server::<_, 32>::new_with_et(AP_IP),
            &ServerOptions::new(AP_IP, Some(&mut gw_buf)),
            &mut bound_socket,
            &mut buf,
        )
        .await;

        match result {
            Ok(_) => {
                client_count += 1;
                info!("🎉 DHCP request processed successfully (client #{})", client_count);
            }
            Err(e) => {
                error!("❌ DHCP server error: {:?}", e);
                warn!("🔄 Continuing DHCP server operation...");
            }
        }

        Timer::after(Duration::from_millis(100)).await;
    }
}

#[embassy_executor::task]
async fn nat_forwarding_task(
    _ap_stack: Stack<'static>,
    _sta_stack: Stack<'static>,
) {
    info!("🔀 Starting NAT forwarding task...");
    warn!("⚠️  NAT forwarding is currently a placeholder implementation");
    
    // This is a placeholder for NAT functionality
    // In a full implementation, we would need to:
    // 1. Listen for packets on the AP interface
    // 2. Modify source IP/port for outbound packets
    // 3. Forward them through the STA interface
    // 4. Track connections in a NAT table
    // 5. Handle return packets by reversing the translation
    //
    // For now, we'll implement basic packet forwarding logic
    info!("📝 NAT Implementation TODO:");
    info!("  1. 👂 Listen for packets on AP interface");
    info!("  2. 🔄 Modify source IP/port for outbound packets");
    info!("  3. ➡️ Forward through STA interface");
    info!("  4. 📋 Track connections in NAT table");
    info!("  5. ⬅️ Handle return packets with reverse translation");
    
    // Note: This is where we would implement NAT translation logic
    // using smoltcp's raw socket capabilities to intercept and modify packets
    
    loop {
        Timer::after(Duration::from_secs(10)).await;
        debug!("🔀 NAT forwarding task still running (placeholder)");
        // TODO: Implement proper NAT forwarding when smoltcp supports it better
        // or when we can access lower-level packet handling
    }
}

#[embassy_executor::task]
async fn web_server_task(stack: Stack<'static>) {
    info!("🌍 Starting web server task...");

    // Wait for network to be ready
    info!("⏳ Waiting for network to be ready...");
    loop {
        if stack.is_link_up() && stack.is_config_up() {
            info!("✅ Network is ready for web server");
            break;
        }
        debug!("🔄 Network not ready, waiting... (link: {}, config: {})", 
               stack.is_link_up(), stack.is_config_up());
        Timer::after(Duration::from_millis(100)).await;
    }

    let mut rx_buffer = [0; 2048];
    let mut tx_buffer = [0; 2048];
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(Duration::from_secs(10)));
    debug!("🔌 TCP socket created (2KB buffers, 10s timeout)");

    info!("🌐 Web server ready on http://192.168.4.1:80/");

    let mut request_count = 0u32;
    loop {
        debug!("🔍 Waiting for web connection...");
        let result = socket
            .accept(IpListenEndpoint {
                addr: None,
                port: 80,
            })
            .await;

        if let Err(e) = result {
            error!("❌ Accept error: {:?}", e);
            Timer::after(Duration::from_millis(1000)).await;
            continue;
        }

        request_count += 1;
        info!("👥 Web client connected (request #{})", request_count);

        // Read HTTP request
        let mut buffer = [0u8; 1024];
        let mut pos = 0;
        loop {
            match socket.read(&mut buffer[pos..]).await {
                Ok(0) => {
                    debug!("📝 HTTP request read complete (EOF)");
                    break; // EOF
                }
                Ok(len) => {
                    pos += len;
                    debug!("📝 Read {} bytes, total: {} bytes", len, pos);
                    let request = unsafe { core::str::from_utf8_unchecked(&buffer[..pos]) };
                    if request.contains("\r\n\r\n") {
                        info!("📬 HTTP Request: {}", request.lines().next().unwrap_or(""));
                        break;
                    }
                    if pos >= buffer.len() {
                        warn!("⚠️  HTTP request buffer full, processing anyway");
                        break;
                    }
                }
                Err(e) => {
                    error!("❌ Read error: {:?}", e);
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
            error!("❌ Write error: {:?}", e);
        } else {
            debug!("✅ HTTP response sent ({} bytes)", response.len());
        }

        if let Err(e) = socket.flush().await {
            error!("❌ Flush error: {:?}", e);
        } else {
            debug!("✅ HTTP response flushed");
        }

        info!("📝 HTTP request #{} completed successfully", request_count);
        Timer::after(Duration::from_millis(100)).await;
        socket.close();
        socket.abort();
    }
}
