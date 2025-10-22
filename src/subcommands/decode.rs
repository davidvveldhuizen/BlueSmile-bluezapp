use crate::ble::telegram::Telegram;
use crate::protocol::{CommandType, ControlCommand};
use anyhow::Result;

pub fn main(data: Vec<String>, format: bool) -> Result<()> {
    let cmd = ControlCommand::new(CommandType::PASSKEY, 123456u32.to_le_bytes());
    println!("cmd: {:?}", cmd.serialize());

    let bytes: Vec<u8> = data
        .as_slice()
        .iter()
        .map(|v| u8::from_str_radix(v, 16).unwrap())
        .collect();

    match Telegram::from_bytes(&bytes) {
        Ok(command) => {
            if format {
                println!(
                    "Telegram {{ \n      device_type: {},\n      serial_number: {},\n      command: Command::{:?},\n      subcommand: {},\n      data: vec!{:?}\n}}",
                    command.device_type,
                    command.serial_number,
                    command.command,
                    command.subcommand,
                    command.data.as_slice(),
                )
            } else {
                println!(
                    "device type: {}\nserial number: {}\ncommand: {:?}\nsubcommand: {}\ndata: {:?}",
                    command.device_type,
                    command.serial_number,
                    command.command,
                    command.subcommand,
                    command.data.as_slice()
                )
            }
        }
        Err(e) => println!("Failed to decode: {}", e),
    }
    Ok(())
}
