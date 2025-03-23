use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod mempool;
mod stream;
use mempool::{MempoolError, read_mempool_from_path};

#[derive(Parser)]
#[command(author, version, about = "Bitcoin Core mempool.dat file parser")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to mempool.dat
    #[arg(short, long, default_value = "mempool.dat")]
    file: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Show mempool dump header info
    Header,

    /// List transactions (limited to first N)
    Decode {
        /// Number of transactions to display
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
        /// Print with Rusts (default) compact debug formatting
        #[clap(long, short)]
        compact: bool,
    },
}

fn main() -> Result<(), MempoolError> {
    let cli = Cli::parse();
    let mempool = read_mempool_from_path(&cli.file)?;

    match cli.command {
        Some(Commands::Header) => {
            let header = mempool.get_file_header();
            println!("{}", header);
        }
        Some(Commands::Decode { limit, compact }) => {
            let entries = mempool.get_mempool_entries();
            let count = entries.len().min(limit);

            for (i, entry) in entries.iter().take(count).enumerate() {
                if compact {
                    println!("[{}] {}", i, entry);
                } else {
                    println!("[{}] {:#}", i, entry);
                }
            }
        }
        None => {}
    }

    Ok(())
}
