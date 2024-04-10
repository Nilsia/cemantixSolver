use cemantix_ia::options::options;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut cli = <options::Cli as clap::Parser>::parse();
    // env::set_var("RUST_BACKTRACE", "1");

    cli.matching().await;

    Ok(())
}
