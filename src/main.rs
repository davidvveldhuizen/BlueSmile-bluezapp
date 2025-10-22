use anyhow::Result;
use cargo_ble::args::{CliArgs, Command};
use cargo_ble::subcommands;
use clap::Parser;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = CliArgs::parse();

    match args.subcommand {
        Command::Run { iterations, delay } => subcommands::run::main(iterations, delay).await,
        Command::AssignPasskey { passkey } => subcommands::assign_passkey::main(passkey).await,
        Command::AssignBaudrate { baudrate } => subcommands::assign_baudrate::main(baudrate).await,
        Command::Decode { bytes, format } => subcommands::decode::main(bytes, format),
        Command::Scan => subcommands::scan::main().await,
        Command::Explore => subcommands::explore::main().await,
        Command::Devices => subcommands::devices::main().await,
    }
}
