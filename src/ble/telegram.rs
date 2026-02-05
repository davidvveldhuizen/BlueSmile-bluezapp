use std::fmt::Display;

use crc::{Crc, CRC_16_MODBUS};
use serde::Serialize;

#[derive(Debug, PartialEq, Clone)]
pub struct Telegram {
    pub device_type: u16,
    pub serial_number: u32,
    pub command: Command,
    pub subcommand: u8,
    pub data: Vec<u8>,
}

#[derive(Debug, PartialEq, Serialize, Clone, Copy)]
pub enum Command {
    Read = 1,
    Write = 2,
    Execute = 3,
}
impl Command {
    pub fn from_byte(byte: u8) -> Result<Self, &'static str> {
        match byte {
            1 => Ok(Self::Read),
            2 => Ok(Self::Write),
            3 => Ok(Self::Execute),
            _ => Err("invallid command byte"),
        }
    }
}

impl Telegram {
    pub fn to_bytes(&self) -> Result<Vec<u8>, &'static str> {
        let max_length: usize = 255;

        if self.data.len() > max_length {
            return Err("data length exceeds maximum allowed size of 255");
        }

        const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);

        let mut buffer = Vec::with_capacity(max_length);

        buffer.extend(&self.device_type.to_be_bytes());
        buffer.extend(&self.serial_number.to_be_bytes());
        buffer.push(4 + self.data.len() as u8);
        buffer.push(self.command as u8);
        buffer.push(self.subcommand);
        buffer.extend(&self.data);
        buffer.extend(CRC.checksum(buffer.as_slice()).to_le_bytes());

        Ok(buffer)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        let len = bytes.len();
        if len < 9 {
            return Err("length to low");
        }
        if len > 255 {
            return Err("length to high");
        }

        const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);

        let checksum = u16::from_le_bytes([bytes[len - 2], bytes[len - 1]]);
        let expected_checksum = CRC.checksum(&bytes[..len - 2]);

        if checksum != expected_checksum {
            return Err("Invallid checksum");
        }

        let telegram = Telegram {
            device_type: u16::from_be_bytes([bytes[0], bytes[1]]),
            serial_number: u32::from_be_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
            command: Command::from_byte(bytes[7])?,
            subcommand: bytes[8],
            data: bytes[9..len - 2].to_vec(),
        };
        Ok(telegram)
    }
}
impl Display for Telegram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "device type: {}\nserial number: {}\ncommand: {:?}\nsubcommand: {}\ndata: {:?}",
            &self.device_type,
            &self.serial_number,
            &self.command,
            &self.subcommand,
            &self.data.as_slice()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_bytes() {
        assert_eq!(
            Telegram {
                device_type: 3730,
                serial_number: 0xFFFFFFFF,
                command: Command::Read,
                subcommand: 204,
                data: Vec::new(),
            }
            .to_bytes(),
            Ok(vec![
                0x0E, 0x92, 0xFF, 0xFF, 0xFF, 0xFF, 0x04, 0x01, 0xCC, 0xB1, 0x21
            ])
        );
    }

    #[test]
    fn test_from_bytes() {
        println!(
            "{}",
            Telegram::from_bytes(&[
                0x0E, 0x92, 0x00, 0x7B, 0x9E, 0x98, 0x06, 0x01, 0xD2, 0x00, 0x31, 0x58, 0xEC
            ])
            .unwrap()
        );
        assert_eq!(
            Telegram::from_bytes(&[
                0x0E, 0x92, 0xFF, 0xFF, 0xFF, 0xFF, 0x04, 0x01, 0xCC, 0xB1, 0x21
            ]),
            Ok(Telegram {
                device_type: 3730,
                serial_number: 0xFFFFFFFF,
                command: Command::Read,
                subcommand: 204,
                data: Vec::new(),
            })
        );
    }
}
