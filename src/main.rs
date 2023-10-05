// hostname -I

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct DeviceState {
    n_pulses: u16,
    pressure_mv: u16,
    a0: u16,
    a1: u16,
    a2: u16,
    a3: u16,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct DeviceControl {
    enable_charger: bool,
    enable_pressure: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case", untagged)]
enum DeviceMessage {
    ToCloud(DeviceState),
    ToDevice(DeviceControl),
}

use csv::Writer;
use rumqttd::{Broker, Config, Notification};
use std::thread;

fn main() {
    // As examples are compiled as seperate binary so this config is current path dependent. Run it
    // from root of this crate
    let config = config::Config::builder()
        .add_source(config::File::with_name("rumqttd.toml"))
        .build()
        .unwrap();

    let config: Config = config.try_deserialize().unwrap();

    // dbg!(&config);

    let mut broker = Broker::new(config);
    let (mut link_tx, mut link_rx) = broker.link("singlenode").unwrap();
    thread::spawn(move || {
        broker.start().unwrap();
    });

    link_tx.subscribe("#").unwrap();

    let mut wtr = Writer::from_path("./result.csv").unwrap();

    loop {
        let notification = match link_rx.recv().unwrap() {
            Some(v) => v,
            None => continue,
        };

        match notification {
            Notification::Forward(forward) => {
                let message: DeviceMessage =
                    serde_json::from_slice(&forward.publish.payload).unwrap();
                match message {
                    DeviceMessage::ToCloud(v) => {
                        wtr.serialize(&v).unwrap();
                        wtr.flush().unwrap();
                        println!("Topic = {:?}, Payload = {:?}", forward.publish.topic, v);
                    }
                    _ => {}
                };
            }
            v => {
                println!("{v:?}");
            }
        }
    }
}
