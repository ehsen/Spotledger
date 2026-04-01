use spotledger::cli::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use clap::Parser;
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve(args) => spotledger::server::serve(args).await,
    }
}
