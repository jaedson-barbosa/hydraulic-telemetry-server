// hostname -I

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct DeviceState {
    n_pulses: u32,
    pressure_ma: f32,
    generator_v: f32,
    battery_v: f32,
    regulator_output_v: f32
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

use rumqttd::{Broker, Config, Notification};
use csv::Writer;
use std::thread;

fn main() {
    // As examples are compiled as seperate binary so this config is current path dependent. Run it
    // from root of this crate
    let config = config::Config::builder()
        .add_source(config::File::with_name("rumqttd.toml"))
        .build()
        .unwrap();

    let config: Config = config.try_deserialize().unwrap();

    dbg!(&config);

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
                let message: DeviceMessage = serde_json::from_slice(&forward.publish.payload).unwrap();
                wtr.serialize(&message).unwrap();
                wtr.flush().unwrap();
                println!(
                    "Topic = {:?}, Payload = {:?}",
                    forward.publish.topic,
                    message
                );
            }
            v => {
                println!("{v:?}");
            }
        }
    }
}
