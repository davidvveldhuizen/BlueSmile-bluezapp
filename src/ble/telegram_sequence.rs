use colored::Colorize;
use std::time::Duration;

use bluer::gatt::remote::{Characteristic, CharacteristicWriteRequest};
use futures::{pin_mut, StreamExt};
use tokio::time::{sleep, timeout};

use crate::ble::telegram::Telegram;

pub struct EventSequence {
    pub sequence: Vec<Telegram>,
    pub delay: Duration,
}

impl EventSequence {
    pub async fn send(&self, char: Characteristic) -> bluer::Result<()> {
        let write_req = CharacteristicWriteRequest {
            op_type: bluer::gatt::WriteOp::Request,
            ..Default::default()
        };

        let notify = char.notify().await?;
        pin_mut!(notify);

        println!(
            "Starting write sequence ({:?}, {})",
            self.delay,
            self.sequence.len()
        );

        let mut first = true;
        for telegram in self.sequence.as_slice() {
            if first {
                first = false;
            } else {
                sleep(self.delay).await;
            }

            print!("{}: {}...", "Request".blue(), telegram);
            let bytes = telegram.to_bytes().unwrap();
            println!("done");

            char.write_ext(&bytes, &write_req).await?;

            match timeout(Duration::from_millis(1500), notify.next()).await {
                Ok(Some(v)) => match Telegram::from_bytes(v.as_slice()) {
                    Ok(r) => println!("{}: {}", "Response".green(), r),
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
            println!();
        }
        Ok(())
    }
}
