// hostname -I

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct DeviceState {
    n_pulses: u16,
    generator_mv: u16,
    battery_mv: u16,
    pressure_mv: u16,
    pressure_en: bool,
    buck_dc: u8,
}

use axum::Router;
use csv::Writer;
use rumqttd::{Broker, Config, Notification};
use std::thread;
use tower_http::services::ServeDir;

async fn serve_dir() {
    let app = Router::new().nest_service("/", ServeDir::new("./"));
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 80));

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
