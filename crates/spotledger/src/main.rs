use spotledger::cli::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use clap::Parser;
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve(args)        => spotledger::server::serve(args).await,
        Commands::NewSite(args)      => spotledger::new_site::new_site(args).await,
        Commands::Migrate(args)      => spotledger::migrate::migrate(args).await,
        Commands::SeedDoctypes(args) => spotledger::seed_doctypes::seed_doctypes(args).await,
        Commands::InstallApp(args)   => spotledger::install_app::install_app(args).await,
        Commands::Emit(args)         => spotledger::emit::emit(args).await,
        Commands::Cleanup(args)      => spotledger::cleanup::cleanup(args).await,
        Commands::Generate(args)     => spotledger::generate::generate(args).await,
    }
}
