use std::thread;
use tracing_subscriber;
use tracing::info;
use config::Config as FileConfig;
use config::File as ConfigFile;
use rumqttd::{Broker, Config, Notification};

fn main() {
    // initialize logging
    tracing_subscriber::fmt().init();

    // load broker configuration
    let file_cfg = FileConfig::builder()
        .add_source(ConfigFile::with_name("rumqttd.toml"))
        .build()
        .unwrap();
    let rumq_cfg: Config = file_cfg.try_deserialize().unwrap();
    let mut broker = Broker::new(rumq_cfg);

    // create a link to receive broker notifications
    let (mut _link_tx, mut link_rx) = broker.link("mqtt-server").unwrap();

    // run the broker in a background thread
    thread::spawn(move || {
        broker.start().unwrap();
    });

    // process notifications (blocking on recv)
    loop {
        match link_rx.recv() {
            Ok(Some(notification)) => {
                info!("Broker notification: {:?}", notification);
            }
            Ok(None) => {
                // link closed, exit loop
                break;
            }
            Err(err) => {
                // channel error, log and exit
                info!("LinkRx recv error: {:?}", err);
                break;
            }
        }
    }
}
