use clap::{Parser, Subcommand};

mod commands;

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
    /// Sync new files from the cloud
    Sync,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Upload { file_path } => {
            commands::upload::run(&file_path).await?;
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
    }

    Ok(())
}
