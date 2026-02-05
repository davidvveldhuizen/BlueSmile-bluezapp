use crate::ble::telegram::Command;
use crate::ble::{find_characteristic, find_device_name, find_service, telegram::Telegram};
use crate::protocol::{CommandType, ControlCommand};
use anyhow::Result;
use bluer::gatt::remote::{Characteristic, CharacteristicWriteRequest};
use bluer::Uuid;
use colored::Colorize;
use dotenv::dotenv;
use futures::{pin_mut, StreamExt};
use std::{env, str::FromStr};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};
use tokio::time::{timeout, Duration};

pub async fn main() -> Result<()> {
    // Get data from .env
    dotenv()?;
    let dev_name = env::var("DEVICE_NAME").expect("DEVICE_NAME not found in .env");
    let service_uuid =
        Uuid::from_str(&env::var("SERVICE_UUID").expect("SERVICE_UUID not found in .env")).unwrap();
    let char_uuid =
        Uuid::from_str(&env::var("TESTBENCH").expect("CHARACTERISTIC not found in .env")).unwrap();
    let ctrl_point_uuid =
        Uuid::from_str(&env::var("CONTROL_POINT").expect("CHARACTERISTIC not found in .env"))
            .unwrap();

    println!("SERVICE: {:?}\nCHARACTER: {:?}\n", service_uuid, char_uuid);

    // Get Device->Service->Character for communication
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;
    println!("addr: {}", adapter.address().await.unwrap());

    let dev = timeout(
        Duration::from_secs(100),
        find_device_name(&adapter, dev_name),
    )
    .await??
    .expect("Couldn't find device address");

    if !dev.is_connected().await? {
        println!("connecting...");
        dev.connect().await?;
    }
    println!("Connected");

    if !dev.is_paired().await? {
        println!("Pairing...");
        match dev.pair().await {
            Ok(_) => {}
            Err(e) => {
                println!("Failed to pair: {}", e);
                dev.disconnect().await.unwrap();
                return Ok(());
            }
        }
    }
    println!("Paired");

    let service = if let Some(s) = find_service(&dev, service_uuid).await? {
        println!("Found service");
        s
    } else {
        return Ok(());
    };
    let char = if let Some(c) = find_characteristic(&service, char_uuid).await? {
        println!("Found Characteristic");
        c
    } else {
        return Ok(());
    };
    let ctrl_point_char: Characteristic =
        if let Some(c) = find_characteristic(&service, ctrl_point_uuid).await? {
            println!("Found Control point");
            c
        } else {
            return Ok(());
        };

    let write_req = CharacteristicWriteRequest {
        op_type: bluer::gatt::WriteOp::Request,
        ..Default::default()
    };

    let notify = char.notify().await?;
    pin_mut!(notify);

    let listener = TcpListener::bind("0.0.0.0:5000").unwrap();

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        println!("Client connected");

        loop {
            let Ok(buf) = tcp_read_telegram(&mut stream) else {
                break;
            };

            let telegram = Telegram::from_bytes(&buf).unwrap();

            print!("{}: {}...", "Request".blue(), telegram);
            char.write_ext(&buf, &write_req).await?;
            println!("done");

            match timeout(Duration::from_millis(1500), notify.next()).await {
                Ok(Some(v)) => match Telegram::from_bytes(v.as_slice()) {
                    Ok(r) => {
                        println!("{}: {}", "Response".green(), r);
                        stream.write_all(&mut v.as_slice()).unwrap();
                    }
                    Err(er) => println!("   Error in response {}", er),
                },
                Ok(None) => println!("    End of messages"),
                Err(e) => println!(
                    "    {}{}{}",
                    "Timeout while reading response, ".yellow(),
                    "Error: ".red(),
                    e
                ),
            }

            if telegram_is_baudrate_change(&telegram) {
                println!("Baudrate change!");
                let mut baudrate_data: [u8; 4] =
                    telegram.data.try_into().expect("baudrate data incorrect");
                baudrate_data.reverse();
                let cmd = ControlCommand::new(CommandType::BAUDRATE, baudrate_data);
                let serialized: Vec<u8> = cmd.serialize();
                let data = serialized.as_slice();

                let ctrl_point_notify = ctrl_point_char.notify().await?;
                pin_mut!(ctrl_point_notify);

                ctrl_point_char.write_ext(data, &write_req).await.unwrap();

                match timeout(Duration::from_millis(1500), ctrl_point_notify.next()).await {
                    Ok(Some(v)) => {
                        let retrieved_baudrate = u32::from_le_bytes([v[0], v[1], v[2], v[3]]);
                        let baudrate_matches: bool = match u32::from_le_bytes(baudrate_data) {
                            2400 => retrieved_baudrate == 0x01a00b,
                            4800 => retrieved_baudrate == 0x00d005,
                            9600 => retrieved_baudrate == 0x006803,
                            14400 => retrieved_baudrate == 0x004507,
                            19200 => retrieved_baudrate == 0x003401,
                            28800 => retrieved_baudrate == 0x00220c,
                            38400 => retrieved_baudrate == 0x001a01,
                            57600 => retrieved_baudrate == 0x001106,
                            115200 => retrieved_baudrate == 0x00080b,
                            _ => false,
                        };
                        if baudrate_matches {
                            println!("new baudrate succesfull");
                        } else {
                            eprintln!(
                                "baudrate failed to assign: retreived wrong baudrate: {}",
                                retrieved_baudrate
                            );
                        }
                    }
                    Ok(None) => println!("    End of messages"),
                    Err(e) => println!("    Timeout while reading response, Error: {}", e),
                }
            }
        }
    }

    Ok(())
}

fn tcp_read_telegram(stream: &mut TcpStream) -> Result<Vec<u8>, ()> {
    let mut len_buf = [0u8; 2];
    if stream.read_exact(&mut len_buf).is_err() {
        println!("Client disconnected");
        return Err(());
    }

    let len = u16::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];

    if stream.read_exact(&mut buf).is_err() {
        println!("Failed to read packet");
        return Err(());
    }
    println!("TCP Paket Received");
    Ok(buf)
}

fn telegram_is_baudrate_change(t: &Telegram) -> bool {
    t.command == Command::Write && t.subcommand == 210
}
