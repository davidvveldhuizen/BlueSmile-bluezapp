use crc::{Crc, CRC_16_MODBUS};
use serde::Serialize;

const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);

#[derive(Copy, Clone, Serialize)]
pub enum CommandType {
    PASSKEY,
    BAUDRATE,
}

impl CommandType {
    pub fn serialize(&self) -> u8 {
        *self as u8 + 1
    }
}

pub struct ControlCommand {
    command_type: CommandType,
    data: [u8; 4],
}

impl ControlCommand {
    pub fn serialize(&self) -> Vec<u8> {
        let mut data: Vec<u8> = Vec::new();
        data.push(self.command_type.serialize());
        data.extend_from_slice(&self.data[..]);
        let crc = CRC.checksum(data.as_slice()).to_le_bytes();
        data.extend_from_slice(&crc[..]);

        data
    }

    pub fn new(command_type: CommandType, data: [u8; 4]) -> Self {
        ControlCommand { command_type, data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_command() {
        let ctrl_cmd = ControlCommand::new(CommandType::PASSKEY, [1, 2, 3, 4]);
        let serialized = ctrl_cmd.serialize();
        assert_eq!(vec![1], serialized);
    }
}
