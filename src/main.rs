// hostname -I

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug)]
pub struct ChargerState {
    pwm_dc: u32,
    max_dc: u32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug)]
pub struct DigitalInputState {
    pub wifi_en: bool,
    pub pressure_en: bool,
    pub charger_en: bool,
    pub high_freq_en: bool
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug)]
pub struct I2CADCRead {
    battery_ma: u16,
    battery_mv: u16,
    esp_vin_mv: u16,
    buck_out_mv: u16,
    pressure_mv: u16
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug)]
pub struct IntADCState {
    pub gpio32: u16,
    pub gpio34: u16,
    pub gpio35: u16,
    pub gpio36: u16,
    pub gpio39: u16,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, strum::FromRepr)]
#[repr(u8)]
pub enum WiFiState {
    Disabled,
    Connecting,
    Connected,
    Error,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug)]
pub struct DeviceState {
    pub charger_state: ChargerState,
    pub digital_state: DigitalInputState,
    pub i2c_adc_state: I2CADCRead,
    pub int_adc_state: IntADCState,
    pub wifi_state: WiFiState,
    pub n_pulses: u16,
    pub register_time_ms: u64,
    pub transmission_time_ms: u64
}

use axum::Router;
use csv::Writer;
use rumqttd::{Broker, Config, Notification};
use std::thread;
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

    let mut wtr = Writer::from_path("./result.csv").unwrap();

    loop {
        let notification = match link_rx.recv().unwrap() {
            Some(v) => v,
            None => continue,
        };

        match notification {
            Notification::Forward(forward) => {
                let v: DeviceState = serde_json::from_slice(&forward.publish.payload).unwrap();
                wtr.serialize(&v).unwrap();
                wtr.flush().unwrap();
                println!("Topic = {:?}, Payload = {:?}", forward.publish.topic, v);
            }
            v => {
                println!("{v:?}");
            }
        }
    }
}
