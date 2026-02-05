use crate::{
    ble::{find_characteristic, find_device_name, find_service},
    protocol::{CommandType, ControlCommand},
};
use anyhow::Result;
use bluer::{gatt::remote::CharacteristicWriteRequest, Uuid};
use dotenv::dotenv;
use futures::{pin_mut, StreamExt};
use std::{env, str::FromStr, time::Duration};
use tokio::time::{sleep, timeout};

pub async fn main(baudrate: u32) -> Result<()> {
    match baudrate {
        4800 => {}
        9600 => {}
        115200 => {}
        _ => {
            println!("Baudrate not supported");
        }
    }

    dotenv()?;
    let dev_name = env::var("DEVICE_NAME").expect("DEVICE_NAME not found in .env");
    println!("device name: {}", dev_name);

    let service_uuid =
        Uuid::from_str(&env::var("SERVICE_UUID").expect("SERVICE_UUID not found in .env")).unwrap();
    let char_uuid =
        Uuid::from_str(&env::var("CONTROL_POINT").expect("CHARACTERISTIC not found in .env"))
            .unwrap();

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    let dev = timeout(Duration::from_secs(5), find_device_name(&adapter, dev_name))
        .await??
        .expect("Couldn't find device address");

    if !dev.is_connected().await? {
        println!("connecting...");
        dev.connect().await?;
    }
    println!("connected");

    if !dev.is_paired().await? {
        println!("pairing...");
        dev.pair().await?;
    }
    println!("paired");

    sleep(Duration::from_secs(1)).await;

    if let Some(service) = find_service(&dev, service_uuid).await? {
        println!("Found service");
        if let Some(char) = find_characteristic(&service, char_uuid).await? {
            println!("  Found Characteristic");

            let write_req = CharacteristicWriteRequest {
                op_type: bluer::gatt::WriteOp::Request,
                ..Default::default()
            };
            let notify = char.notify().await?;
            pin_mut!(notify);

            let cmd = ControlCommand::new(CommandType::BAUDRATE, baudrate.to_le_bytes());
            let serialized: Vec<u8> = cmd.serialize();
            let data = serialized.as_slice();

            char.write_ext(data, &write_req).await?;

            match timeout(Duration::from_millis(1500), notify.next()).await {
                Ok(Some(v)) => {
                    let retrieved_baudrate = u32::from_le_bytes([v[0], v[1], v[2], v[3]]);
                    let baudrate_matches: bool = match baudrate {
                        2400 => retrieved_baudrate == 0x01a00b,
                        4800 => retrieved_baudrate == 0x00d005,
                        9600 => retrieved_baudrate == 0x006803,
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

    dev.disconnect().await?;

    Ok(())
}
