use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect};

pub async fn main() -> Result<()> {
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;
    let devices = adapter.device_addresses().await.unwrap();

    let mut options: Vec<String> = Vec::new();
    for a in &devices {
        let name = adapter
            .device(*a)
            .unwrap()
            .name()
            .await
            .unwrap_or(None)
            .unwrap_or_else(|| "Unknown".to_string());
        options.extend_from_slice(&[format!("{}, {}", a, name)]);
    }

    let res = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select device")
        .items(&options)
        .interact()
        .unwrap();

    let selected_device = devices[res];

    let res = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Remove?")
        .interact()
        .unwrap();

    if res {
        adapter.remove_device(selected_device).await?;
    }

    Ok(())
}
