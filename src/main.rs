use cemantix_ia::options::options::{Cli, LogLevel};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut cli = Cli::parse();
    if let Err(e) = cli.verify() {
        eprintln!("{e}");
        return Ok(());
    }
    cli.log(
        &format!("Executing {} command", cli.command),
        LogLevel::Info,
    )?;

    // env::set_var("RUST_BACKTRACE", "1");

    if let Err(e) = cli.matching().await {
        cli.log_and_print(&e.to_string(), LogLevel::Error)?;
    };

    Ok(())
}
