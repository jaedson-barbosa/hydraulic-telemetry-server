use std::sync::Arc;

use mqttrs::{encode_slice, Packet::*};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::broadcast::{self, Receiver, Sender},
};

// hostname -I

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct DeviceState {
    n_pulses: u32,
    pressure_ma: f32,
    generator_mv: u32,
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

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("0.0.0.0:1883").await.unwrap();
    let (tx, rx) = broadcast::channel::<DeviceMessage>(16);
    let tx = Arc::new(tx);

    tokio::spawn(async move {
        logger(rx).await;
    });

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let sender = (&tx).clone();
        tokio::spawn(async move {
            process(socket, &sender).await;
        });
    }
}

async fn process(mut socket: TcpStream, sender: &Sender<DeviceMessage>) {
    // let receiver = sender.subscribe();

    let mut buf = Vec::new();

    loop {
        buf.clear();
        let length = socket.read_buf(&mut buf).await.unwrap();
        // if length == 0 { break; }
        println!("{length}");
        let decoded = mqttrs::decode_slice(&buf).unwrap();
        let packet = match decoded {
            Some(v) => v,
            None => continue
        };
        match packet {
            Publish(v) => {
                let message: DeviceMessage = serde_json::from_slice(v.payload).unwrap();
                sender.send(message).unwrap();
            }
            Subscribe(v) => {}
            Unsubscribe(v) => {}
            // TODO Implement Auth using OTP Validation as explained in
            // Internet of things (IoT) technologies applications challenges and solutions
            Connect(_) => {
                let response = mqttrs::Connack {
                    session_present: false,
                    code: mqttrs::ConnectReturnCode::Accepted,
                };
                encode_slice(&response.into(), &mut buf).unwrap();
                socket.write(&buf).await.unwrap();
                println!("Connection accepted");
            },
            Pingreq => {
                let response = Pingresp {};
                encode_slice(&response, &mut buf).unwrap();
                socket.write(&buf).await.unwrap();
            }
            other => println!("Other: {other:?}"),
        };
    }
}

async fn logger(mut receiver: Receiver<DeviceMessage>) {
    use csv::Writer;

    let mut wtr = Writer::from_path("./result.csv").unwrap();
    wtr.flush().unwrap();
    loop {
        let state = receiver.recv().await.unwrap();
        wtr.serialize(&state).unwrap();
        println!("{state:?}");
        wtr.flush().unwrap();
    }
}
