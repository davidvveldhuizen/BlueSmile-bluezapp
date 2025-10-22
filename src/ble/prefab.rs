use std::{
    fs::File,
    io::{BufRead, BufReader},
    time::Duration,
};

use crate::ble::{
    telegram::{Command, Telegram},
    telegram_sequence::EventSequence,
};

pub fn get_sequence(num: usize, delay: Duration) -> EventSequence {
    let path = "commands.txt";
    let file = File::open(path).expect("could not find file");
    let reader = BufReader::new(file);

    let mut telegrams = Vec::new();

    let mut i = 0;
    for line in reader.lines() {
        i += 1;
        if i > num {
            break;
        }

        let line = line.unwrap();

        let bytes = line
            .split(' ')
            .map(|s| u8::from_str_radix(s, 16).unwrap())
            .collect::<Vec<u8>>();

        let mut telegram = Telegram::from_bytes(bytes.as_slice()).unwrap();
        telegram.serial_number = 0xFFFFFFFF;

        telegrams.push(telegram);
    }

    EventSequence {
        sequence: telegrams,
        delay,
    }
}

pub fn greet_sequence() -> EventSequence {
    EventSequence {
        sequence: vec![Telegram {
            device_type: 3793,
            serial_number: 0xFFFFFFFF,
            command: Command::Read,
            subcommand: 204,
            data: vec![],
        }],
        delay: Duration::from_secs(1),
    }
}

pub fn small_sequence(num: usize, delay: Duration) -> EventSequence {
    EventSequence {
        sequence: vec![
            Telegram {
                device_type: 3730,
                serial_number: 0xFFFFFFFF,
                command: Command::Read,
                subcommand: 204,
                data: vec![],
            };
            num
        ],
        delay,
    }
}

pub fn big_resp_sequence(num: usize, delay: Duration) -> EventSequence {
    EventSequence {
        sequence: vec![
            Telegram {
                device_type: 3730,
                serial_number: 0xFFFFFFFF,
                command: Command::Read,
                subcommand: 206,
                data: vec![88, 0, 0, 1],
            };
            num
        ],
        delay,
    }
}
