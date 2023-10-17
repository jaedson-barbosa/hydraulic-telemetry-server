// hostname -I

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug)]
pub struct I2CADCRead {
    battery_ma: i16,
    battery_mv: u16,
    esp_vin_mv: u16,
    generator_mv: u16,
    pressure_mv: u16
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug)]
pub struct DeviceState {
    pub i2c_adc_state: I2CADCRead,
    pub pwm_pct: u8,
    pub n_pulses: u16,
    pub time_ms: u64,
}

use axum::Router;
use rumqttd::{Broker, Config, Notification};
use std::{thread, fs::File, io::{Write, Seek}};
use tower_http::services::ServeDir;

async fn serve_dir() {
    let app = Router::new().nest_service("/", ServeDir::new("./"));
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));

    if let Err(v) = axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
    {
        println!("Error {v:?}")
    }
}

#[tokio::main]
async fn main() {
    // As examples are compiled as seperate binary so this config is current path dependent. Run it
    // from root of this crate
    let config = config::Config::builder()
        .add_source(config::File::with_name("rumqttd.toml"))
        .build()
        .unwrap();

    let config: Config = config.try_deserialize().unwrap();

    tokio::spawn(serve_dir());
    // dbg!(&config);

    let mut broker = Broker::new(config);
    let (mut link_tx, mut link_rx) = broker.link("singlenode").unwrap();
    thread::spawn(move || {
        broker.start().unwrap();
    });

    link_tx.subscribe("#").unwrap();

    let mut file = File::create("./result.json").unwrap();
    file.write("[]".as_bytes()).unwrap();
    file.seek(std::io::SeekFrom::Start(0)).unwrap();
    let mut first = true;

    loop {
        let notification = match link_rx.recv().unwrap() {
            Some(v) => v,
            None => continue,
        };

        match notification {
            Notification::Forward(forward) => {
                let payload = forward.publish.payload;
                let device_state: DeviceState = match serde_json::from_slice(&payload) {
                    Ok(v) => v,
                    Err(v) => {
                        println!("Error while parsing: {v:?}; Payload: {}", core::str::from_utf8(&payload).unwrap());
                        continue;
                    }
                };
                file.seek(std::io::SeekFrom::End(-1)).unwrap();
                if first {
                    first = false;
                } else {
                    file.write(",\n".as_bytes()).unwrap();
                }
                file.write(&payload).unwrap();
                file.write("]".as_bytes()).unwrap();
                file.flush().unwrap();
                println!("Topic = {:?}, Payload = {:?}", forward.publish.topic, device_state);
            }
            v => {
                println!("{v:?}");
            }
        }
    }
}
