use crate::{
    ble::{find_characteristic, find_device_name, find_device_select, find_service},
    protocol::{CommandType, ControlCommand},
};
use anyhow::Result;
use bluer::{gatt::remote::CharacteristicWriteRequest, Uuid};
use dotenv::dotenv;
use futures::{pin_mut, StreamExt};
use rand::random_range;
use std::{env, str::FromStr, time::Duration};
use tokio::time::{sleep, timeout};

pub async fn main(passkey: Option<u32>) -> Result<()> {
    if let Some(passkey) = passkey {
        if passkey > 999999 {
            panic!("passkey cant be higher then 999999")
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

    let dev = timeout(
        Duration::from_secs(5),
        //find_device_name(&adapter, dev_name))
        find_device_select(&adapter),
    )
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

            let new_passkey: u32 = passkey.unwrap_or(random_range(0..999999));
            println!("new passkey: {}", new_passkey);

            let cmd = ControlCommand::new(CommandType::PASSKEY, new_passkey.to_le_bytes());
            let serialized: Vec<u8> = cmd.serialize();
            let data = serialized.as_slice();

            char.write_ext(data, &write_req).await.expect("ohno");

            match timeout(Duration::from_millis(1500), notify.next()).await {
                Ok(Some(v)) => {
                    println!("{:?}", v);
                    let retrieved_passkey = u32::from_le_bytes([v[0], v[1], v[2], v[3]]);
                    if passkey == Some(retrieved_passkey) {
                        adapter.remove_device(dev.address()).await?;
                        println!("new passkey succesfull");
                    } else {
                        eprintln!(
                            "passkey failed to assign: retreived wrong passkey: {}",
                            retrieved_passkey
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
