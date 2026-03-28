// src/main.rs
use clap::{ Parser, Subcommand };
use better_fs::chunker::chunk_lengths;
use better_fs::file_manager::{FileKind, FileManager};
use better_fs::fuse_handler;
use std::fs;
use std::io::Write; // Needed for flushing output
use std::path::PathBuf;
// use fuser::{ MountOption, Session }; // Unused but kept for future use

// 1. Define the Command Line Interface (CLI)
#[derive(Parser)]
#[command(name = "BetterFS")]
#[command(about = "A deduplicating, content-addressable filesystem", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Save a file to BetterFS
    Write {
        /// The path to the file you want to upload
        file_path: PathBuf,
    },
    /// Read a file back from BetterFS
    Read {
        /// The name of the file inside BetterFS
        file_name: String,
    },
    /// List all files stored in BetterFS
    List,
    Mount {
        /// The folder to mount to (e.g., ./mnt)
        mount_point: String,
    },
    /// Inspect the internal database (for debugging)
    Inspect,
    /// Run Garbage Collection to remove unused chunks
    Gc,
    /// Analyze CDC chunk distribution for a file
    CdcStats {
        /// File path to analyze
        file_path: PathBuf,
    },
}

fn percentile(sorted: &[usize], p: f64) -> usize {
    if sorted.is_empty() {
        return 0;
    }

    let rank = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

fn main() {
    env_logger::init();
    let args = Cli::parse();

    // Initialize the engine in a folder named "my_storage"
    // This creates a permanent database on your disk.
    let storage_path = "./my_storage";
    let manager = FileManager::new(storage_path);

    match args.command {
        Commands::Write { file_path } => {
            // 1. Read data from your REAL hard drive
            let data = match fs::read(&file_path) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Error: Could not read file '{:?}': {}", file_path, e);
                    return;
                }
            };

            let filename = file_path.file_name().unwrap().to_str().unwrap();

            // 2. Ingest it into BetterFS
            println!("Writing '{}' ({} bytes)...", filename, data.len());
            match manager.write_file(filename, &data) {
                Ok(_) => println!("Success! Saved as '{}'", filename),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Read { file_name } => {
            // 1. Ask BetterFS for the bytes
            match manager.read_file(&file_name) {
                Ok(data) => {
                    // 2. Write to Standard Output (so you can pipe it)
                    std::io::stdout().write_all(&data).unwrap();
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::List => {
            let files = manager.list_files();
            if files.is_empty() {
                println!("No files found in storage.");
            } else {
                println!("Files in BetterFS:");
                for file in files {
                    println!(" - {}", file);
                }
            }
        }
        Commands::Mount { mount_point } => {
            println!("Mounting BetterFS to {}...", mount_point);
            println!("(Press Ctrl+C to unmount)");

            // Ensure the mount point exists
            fs::create_dir_all(&mount_point).unwrap();

            // Start the FUSE Driver
            let options = vec![
                fuser::MountOption::RW, // Read-Only
                fuser::MountOption::AllowOther,
                fuser::MountOption::FSName("betterfs".to_string()),
                fuser::MountOption::AutoUnmount // Helps clean up on exit
            ];

            let fs_impl = fuse_handler::BetterFS::new(manager);

            fuser::mount2(fs_impl, mount_point, &options).unwrap();
        }
        Commands::Inspect => {
            println!("--- INSPECTING DATABASE ---");
            for (key, parsed_recipe) in manager.inspect_records() {
                if let Some(recipe) = parsed_recipe {
                    let kind_str = match recipe.kind {
                        FileKind::Directory => "DIR",
                        FileKind::File => "FILE",
                    };
                    println!(
                        "[{}] {} \t(Size: {} bytes, Chunks: {})",
                        kind_str,
                        key,
                        recipe.file_size,
                        recipe.chunks.len()
                    );
                } else {
                    println!("[???] {} \t(Raw Data)", key);
                }
            }
            println!("---------------------------");
        }

        // Garbage Collection Command
        Commands::Gc =>
            match manager.run_gc() {
                Ok(count) => println!("Successfully removed {} orphaned chunks.", count),
                Err(e) => eprintln!("GC Failed: {}", e),
            },

        Commands::CdcStats { file_path } => {
            let data = match fs::read(&file_path) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Error: Could not read file '{:?}': {}", file_path, e);
                    return;
                }
            };

            let sizes = chunk_lengths(&data);
            if sizes.is_empty() {
                println!("No chunks produced (input file is empty).");
                return;
            }

            let mut sorted = sizes.clone();
            sorted.sort_unstable();

            let total_bytes: usize = sizes.iter().sum();
            let avg = total_bytes as f64 / sizes.len() as f64;
            let min = *sorted.first().unwrap_or(&0);
            let max = *sorted.last().unwrap_or(&0);

            println!("CDC Stats for {:?}", file_path);
            println!("- bytes: {}", data.len());
            println!("- chunks: {}", sizes.len());
            println!("- avg: {:.2}", avg);
            println!("- min: {}", min);
            println!("- p50: {}", percentile(&sorted, 0.50));
            println!("- p90: {}", percentile(&sorted, 0.90));
            println!("- p99: {}", percentile(&sorted, 0.99));
            println!("- max: {}", max);
        }
    }
}
