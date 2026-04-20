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
        Commands::Use(args)          => spotledger::use_site::use_site(args).await,
        Commands::Start(args)        => spotledger::start::start(args).await,
        Commands::WireApp(args)      => spotledger::wire_app::wire_app(args).await,
        Commands::NewApp(args)       => spotledger::new_app::new_app(args).await,
        Commands::NewModule(args)    => spotledger::new_app::new_module(args).await,
        Commands::NewDoctype(args)   => spotledger::new_app::new_doctype(args).await,
        Commands::ExportApp(args)    => spotledger::new_app::export_app(args).await,
        Commands::PackApp(args)      => spotledger::pack_app::pack_app(args).await,
        Commands::SeedFinanceFixtures(args) => {
            spotledger::seed_finance_fixtures::seed_finance_fixtures(args).await
        }
    }
}
