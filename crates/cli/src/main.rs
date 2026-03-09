use clap::{Parser, Subcommand};

mod api;
mod commands;
mod config;
mod key;

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
        /// Override the remote S3 key (default: <filename>.enc)
        #[arg(long)]
        remote_path: Option<String>,
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

    match cli.command {
        Commands::Upload {
            file_path,
            remote_path,
        } => {
            commands::upload::run(&file_path, remote_path.as_deref()).await?;
        }
        Commands::Download { remote_path } => {
            commands::download::run(&remote_path).await?;
        }
        Commands::List { prefix } => {
            commands::list::run(prefix.as_deref()).await?;
        }
        Commands::Sync => {
            commands::sync::run().await?;
        }
        Commands::Delete { remote_path } => {
            commands::delete::run(&remote_path).await?;
        }
        Commands::Move { from, to } => {
            commands::move_cmd::run(&from, &to).await?;
        }
    }

    Ok(())
}
