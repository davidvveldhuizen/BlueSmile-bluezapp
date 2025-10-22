pub mod prefab;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use futures::stream;
use futures::{pin_mut, StreamExt};
use tokio::{
    io::{self, AsyncBufReadExt},
    select,
};
pub mod telegram;
pub mod telegram_sequence;
use bluer::{Adapter, Address, Device, DiscoveryFilter, DiscoveryTransport, Uuid};

pub async fn find_device(adapter: &Adapter, addr: Address) -> bluer::Result<Option<Device>> {
    let filter = DiscoveryFilter {
        transport: DiscoveryTransport::Le,
        ..Default::default()
    };
    adapter.set_discovery_filter(filter).await?;

    let device_events = adapter.discover_devices().await?;
    pin_mut!(device_events);

    println!("Searching for device with addres {}", addr);
    while let Some(device_event) = device_events.next().await {
        match device_event {
            bluer::AdapterEvent::DeviceAdded(dev_addr) => {
                let dev = adapter.device(dev_addr)?;
                let name = dev.name().await?;
                println!("  Device added {}, {:?}", dev_addr, name);
                if dev_addr == addr {
                    println!("Found device  {}, {:?}", dev_addr, name);
                    return Ok(Some(dev));
                }
            }
            bluer::AdapterEvent::DeviceRemoved(dev_addr) => {
                let dev = adapter.device(dev_addr)?;
                let name = dev.name().await;
                println!("  Device removed {}, {:?}", dev_addr, name);
            }
            _ => {}
        }
    }
    Ok(None)
}

pub async fn find_device_name(adapter: &Adapter, name: String) -> bluer::Result<Option<Device>> {
    let filter = DiscoveryFilter {
        transport: DiscoveryTransport::Le,
        ..Default::default()
    };
    adapter.set_discovery_filter(filter).await?;

    let device_events = adapter.discover_devices().await?;
    pin_mut!(device_events);

    println!("Searching for device with name {}...", name);
    while let Some(device_event) = device_events.next().await {
        if let bluer::AdapterEvent::DeviceAdded(dev_addr) = device_event {
            let dev = adapter.device(dev_addr)?;
            let dev_name_opt = dev.name().await?;
            if let Some(dev_name) = dev_name_opt {
                if dev_name == name {
                    println!("Found device  {}, {:?}", dev_addr, name);
                    return Ok(Some(dev));
                }
            }
        }
    }
    Ok(None)
}

pub async fn find_device_select(adapter: &Adapter) -> bluer::Result<Option<Device>> {
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

    Ok(Some(device))
}

pub async fn find_service(
    dev: &Device,
    uuid: Uuid,
) -> bluer::Result<Option<bluer::gatt::remote::Service>> {
    for service in dev.services().await? {
        if service.uuid().await? == uuid {
            return Ok(Some(service));
        }
    }
    Ok(None)
}

pub async fn find_characteristic(
    service: &bluer::gatt::remote::Service,
    uuid: Uuid,
) -> bluer::Result<Option<bluer::gatt::remote::Characteristic>> {
    for char in service.characteristics().await? {
        if char.uuid().await? == uuid {
            return Ok(Some(char));
        }
    }
    Ok(None)
}
