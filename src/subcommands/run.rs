use crate::ble::telegram::Telegram;
use crate::ble::telegram_sequence::EventSequence;
use crate::ble::{find_characteristic, find_service, prefab};
use crate::ble::{find_device_name, find_device_select};
use anyhow::Result;
use bluer::Uuid;
use dotenv::dotenv;
use std::{env, str::FromStr};
use tokio::time::{sleep, timeout, Duration};

pub async fn main(send_amount: usize, delay: u64) -> Result<()> {
    // Get data from .env
    dotenv()?;
    let dev_name = env::var("DEVICE_NAME").expect("DEVICE_NAME not found in .env");
    let service_uuid =
        Uuid::from_str(&env::var("SERVICE_UUID").expect("SERVICE_UUID not found in .env")).unwrap();
    let char_uuid =
        Uuid::from_str(&env::var("TESTBENCH").expect("CHARACTERISTIC not found in .env")).unwrap();

    println!(
        "SERVICE: {:?}\nCHARACTER: {:?}\ndelay: {:?}",
        service_uuid, char_uuid, delay
    );

    // Get Device->Service->Character for communication
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;
    println!("addr: {}", adapter.address().await.unwrap());

    let dev = timeout(
        Duration::from_secs(100),
        find_device_name(&adapter, dev_name),
        // find_device_select(&adapter),
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
            Ok(_) => println!("Paired"),
            Err(e) => {
                println!("Failed to pair: {}", e);
                dev.disconnect().await.unwrap();
                return Ok(());
            }
        }
    }

    sleep(Duration::from_secs(1)).await;

    // let sequence = prefab::get_sequence(send_amount, Duration::from_millis(delay));
    let sequence = EventSequence {
        sequence: vec![
            Telegram {
                device_type: 0xffff,
                serial_number: 0xffffffff,
                command: crate::ble::telegram::Command::Read,
                subcommand: 101,
                data: vec![], //vec![0xAA; 244],
            };
            send_amount
        ],
        delay: Duration::from_millis(delay),
    };
    println!(">>{:?}<<", sequence.sequence[0].to_bytes().unwrap());

    if let Some(service) = find_service(&dev, service_uuid).await? {
        println!("Found service");
        if let Some(char) = find_characteristic(&service, char_uuid).await? {
            println!("  Found Characteristic");

            sequence.send(char).await?;
        }
    }

    sleep(Duration::from_millis(100)).await;

    dev.disconnect().await?;
    println!("disconnected");

    Ok(())
}
