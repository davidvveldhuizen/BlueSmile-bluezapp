use clap::{Parser, Subcommand};
#[derive(Parser, Debug)]
#[command(name = "ble")]
pub struct CliArgs {
    #[clap(subcommand)]
    pub subcommand: Command,
}

#[derive(Debug, Subcommand)]
#[command(name = "ble", about = "CLI build for BlueSmile project")]
pub enum Command {
    #[command(about = "runs a sequence of messages and reads the responses")]
    Run {
        iterations: usize,
        delay: u64,
    },
    #[command(about = "assign new passkey to ble-module")]
    AssignPasskey {
        passkey: Option<u32>,
    },
    #[command(about = "assign new passkey to ble-module")]
    AssignBaudrate {
        baudrate: u32,
    },
    #[command(about = "decodes bytes to a telegram")]
    Decode {
        bytes: Vec<String>,
        #[arg(long, short)]
        format: bool,
    },
    #[command(about = "scan for devices")]
    Scan,
    Explore,
    #[command(about = "manage devices")]
    Devices,
    #[command(about = "Passes data between BT module and TCP")]
    PassThrough,
}
