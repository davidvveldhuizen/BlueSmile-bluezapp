use std::time::Duration;

use anyhow::Result;
use bluer::{
    gatt::{
        remote::{Characteristic, CharacteristicWriteRequest, Service},
        CharacteristicFlags,
    },
    Device, DiscoveryFilter, DiscoveryTransport,
};
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use futures::stream;
use futures::{pin_mut, StreamExt};
use tokio::{
    io::{self, AsyncBufReadExt},
    select,
    time::{sleep, timeout},
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

    let dev = devices[res].clone();

    loop {
        let mut options: Vec<&str> = Vec::new();

        if dev.is_connected().await.unwrap() {
            options.push("Disconnect");
            options.push("Services");

            if !dev.is_paired().await.unwrap() {
                options.push("Pair");
            }
        } else {
            options.extend_from_slice(&["Connect"]);
        }
        if dev.is_trusted().await.unwrap() {
            options.push("Forget");
        }
        options.push("Info");
        options.push("Quit");

        let res = FuzzySelect::with_theme(&ColorfulTheme::default())
            .items(options.as_slice())
            .interact()
            .unwrap();

        match options[res] {
            "Connect" => dev.connect().await?,
            "Disconnect" => dev.disconnect().await?,
            "Pair" => match dev.pair().await {
                Ok(_) => {}
                Err(_) => {
                    eprintln!("pair failed");
                    sleep(Duration::from_secs(2)).await;
                    match dev.is_connected().await {
                        Ok(c) => {
                            if c {
                                if let Err(e) = dev.disconnect().await {
                                    adapter.set_powered(false).await?;
                                    eprintln!("Failed to disconnect: {}", e);
                                    break;
                                }
                            }
                        }
                        Err(e) => eprintln!("Failed checking connection: {}", e),
                    }
                }
            },
            "Services" => services_menu(&dev).await,
            "Forget" => adapter.remove_device(dev.address()).await?,
            "Info" => print_dev_info(&dev).await,
            "Quit" => break,
            _ => break,
        }
    }

    dev.disconnect().await.unwrap();
    Ok(())
}

async fn services_menu(dev: &Device) {
    let services: Vec<Service> = dev.services().await.unwrap();
    let mut options: Vec<String> = stream::iter(services.clone())
        .then(|s| async move { format!("{}", s.uuid().await.unwrap()) })
        .collect()
        .await;
    options.push("Back".to_string());

    let res = FuzzySelect::with_theme(&ColorfulTheme::default())
        .items(options.as_slice())
        .interact()
        .unwrap();

    if res != services.len() {
        chars_menu(&services[res]).await;
    }
}

async fn chars_menu(serv: &Service) {
    let chars = serv.characteristics().await.unwrap();
    let mut options: Vec<String> = stream::iter(chars.clone())
        .then(|s| async move { format!("{}", s.uuid().await.unwrap()) })
        .collect()
        .await;
    options.push("Back".to_string());

    let res = FuzzySelect::with_theme(&ColorfulTheme::default())
        .items(options.as_slice())
        .interact()
        .unwrap();

    if res != chars.len() {
        char_menu(&chars[res]).await;
    }
}

async fn char_menu(char: &Characteristic) {
    let options = vec!["Write", "Read", "Read Response", "Back"];

    let write_req = CharacteristicWriteRequest {
        op_type: bluer::gatt::WriteOp::Request,
        ..Default::default()
    };

    let notify = char.notify().await.unwrap();
    pin_mut!(notify);

    loop {
        let res = FuzzySelect::with_theme(&ColorfulTheme::default())
            .items(options.as_slice())
            .interact()
            .unwrap();

        match options[res] {
            "Write" => {
                let input: String = dialoguer::Input::with_theme(&ColorfulTheme::default())
                    .interact()
                    .unwrap();
                let bytes: Vec<u8> = input
                    .split(&[' ', ','])
                    .map(|s| {
                        if let Some(s) = s.strip_prefix("0x") {
                            u8::from_str_radix(s, 16).unwrap()
                        } else {
                            print!("{}", s);
                            s.parse::<u8>().unwrap()
                        }
                    })
                    .collect();
                char.write_ext(bytes.as_slice(), &write_req).await.unwrap();
            }
            "Read" => println!("{:?}", char.read().await.unwrap()),
            "Read Response" => println!(
                "response: {:?}",
                timeout(Duration::from_millis(1500), notify.next()).await
            ),
            "Back" => break,
            _ => {}
        }
    }
}

async fn print_dev_info(device: &Device) {
    println!(
        "name: {:?}
address: {}
advertising flags: {:?}
advertinsing data: {:?}
uuids: {:?}
manufacturer data: {:?}
battery percentage: {:?}
class: {:?}
service data: {:?}
signal strength: {:?}",
        device.name().await.unwrap(),
        device.address(),
        device.advertising_flags().await.unwrap(),
        device.manufacturer_data().await.unwrap(),
        device.uuids().await.unwrap(),
        device.manufacturer_data().await.unwrap(),
        device.battery_percentage().await.unwrap(),
        device.class().await.unwrap(),
        device.service_data().await.unwrap(),
        device.rssi().await.unwrap(),
    );
}
