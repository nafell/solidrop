use clap::{Parser, Subcommand};

mod api_client;
mod commands;
mod config;
mod master_key;

#[derive(Parser)]
#[command(
    name = "solidrop",
    version,
    about = "SoliDrop PC CLI — upload, download, and manage files"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Upload a file to the cloud
    Upload {
        /// Path to the file to upload
        file_path: String,
    },
    /// Download a file from the cloud
    Download {
        /// Remote path of the file to download
        remote_path: String,
    },
    /// List files in the cloud
    List {
        /// Filter by path prefix
        #[arg(long)]
        prefix: Option<String>,
    },
    /// Sync new files from the transfer/ prefix
    Sync,
    /// Delete a file from the cloud
    Delete {
        /// Remote path of the file to delete
        remote_path: String,
    },
    /// Move (rename) a file in the cloud
    Move {
        /// Current remote path
        from: String,
        /// New remote path
        to: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let config = config::CliConfig::load()?;
    let api = api_client::ApiClient::from_config(&config)?;

    match cli.command {
        Commands::Upload { file_path } => {
            let key = master_key::acquire_master_key(&config.crypto)?;
            commands::upload::run(&config, &api, &key, &file_path).await?;
        }
        Commands::Download { remote_path } => {
            let key = master_key::acquire_master_key(&config.crypto)?;
            commands::download::run(&config, &api, &key, &remote_path).await?;
        }
        Commands::List { prefix } => {
            commands::list::run(&api, prefix.as_deref()).await?;
        }
        Commands::Sync => {
            let key = master_key::acquire_master_key(&config.crypto)?;
            commands::sync::run(&config, &api, &key).await?;
        }
        Commands::Delete { remote_path } => {
            commands::delete::run(&api, &remote_path).await?;
        }
        Commands::Move { from, to } => {
            commands::move_cmd::run(&api, &from, &to).await?;
        }
    }

    Ok(())
}
