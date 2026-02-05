use anyhow::Result;
use bluer::{Device, DiscoveryFilter, DiscoveryTransport};
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use futures::stream;
use futures::{pin_mut, StreamExt};
use tokio::{
    io::{self, AsyncBufReadExt},
    select,
};

pub async fn main() -> Result<()> {
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    let filter = DiscoveryFilter {
        transport: DiscoveryTransport::Le,
        ..Default::default()
    };
    adapter.set_discovery_filter(filter).await?;

    let device_events = adapter.discover_devices().await?;
    pin_mut!(device_events);

    let mut devices: Vec<Device> = Vec::new();
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    println!("To stop scan, press <ENTER>");

    loop {
        select!(
            Some(dev_event) = device_events.next() => {
                match dev_event {
                    bluer::AdapterEvent::DeviceAdded(dev_addr) => {
                        let dev = adapter.device(dev_addr)?;
                        let name = dev.name().await?;
                        devices.push(dev);
                        println!("  Device added {}, {:?}", dev_addr, name);
                    }
                    bluer::AdapterEvent::DeviceRemoved(dev_addr) => {
                        let dev = adapter.device(dev_addr)?;
                        let name = dev.name().await;
                        devices.retain(|v| v.address() != dev_addr);
                        println!("  Device removed {}, {:?}", dev_addr, name);
                    }
                    _ => {}
                }
            },
            Ok(Some(_)) = stdin.next_line() => break,
        );
    }

    let options: Vec<String> = stream::iter(devices.clone())
        .then(|d| async move {
            let name = d
                .name()
                .await
                .unwrap_or(None)
                .unwrap_or_else(|| "Unknown".to_string());
            format!("{}, {}", d.address(), name)
        })
        .collect()
        .await;

    let res = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select device")
        .items(&options)
        .interact()
        .unwrap();

    let device = devices[res].clone();

    println!(
        "name{:?}
address: {}
advertising flags: {:?}
uuids: {:?}
manufacturer data: {:?} 
battery percentage: {:?}
class: {:?}
service data: {:?}
signal strength: {:?}",
        device
            .name()
            .await
            .unwrap()
            .map_or("UNKNOWN".to_string(), |n| n),
        device.address(),
        device.advertising_flags().await.unwrap(),
        device.uuids().await.unwrap(),
        device.manufacturer_data().await.unwrap(),
        device.battery_percentage().await.unwrap(),
        device.class().await.unwrap(),
        device.service_data().await.unwrap(),
        device.rssi().await.unwrap(),
    );

    let mut device_type: u16;
    let mut serial_number: u32;
    if let Some(mandata) = device.manufacturer_data().await.unwrap() {
        for val in mandata.values() {
            device_type = u16::from_be_bytes([val[0], val[1]]);
            println!("device type: {:?}", device_type);
            serial_number = u32::from_be_bytes([val[2], val[3], val[4], val[5]]);
            println!("serial_number: {:?}", serial_number);
        }
    }

    Ok(())
}
