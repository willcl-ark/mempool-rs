use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod mempool;
mod stream;
mod tui;
use mempool::{MempoolError, read_mempool_from_path};
use tui::TuiApp;

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

    /// Interactive TUI mode with transaction browser
    Interact,
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
        Some(Commands::Interact) => {
            // Format header information for display in the popup
            let header = mempool.get_file_header();

            // Only show XOR key for V2 format
            let xor_key_display = if header.version == 2 {
                match mempool.get_xor_key() {
                    Some(key) => format!("XOR key: {:02x?}", key),
                    None => "XOR key: Not found".to_string(),
                }
            } else {
                "".to_string() // No XOR key in V1 format
            };

            let header_info = format!(
                "Version: {}\nNumber of transactions: {}\n{}",
                header.version, header.num_tx, xor_key_display
            );

            let entries = mempool.get_mempool_entries();
            let mut app = TuiApp::new(entries, header_info);
            if let Err(err) = app.run() {
                eprintln!("Error running TUI: {}", err);
            }
        }
        None => {}
    }

    Ok(())
}
