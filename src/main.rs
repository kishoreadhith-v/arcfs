// src/main.rs
use clap::{ Parser, Subcommand };
use arcfs::chunker::chunk_lengths;
use arcfs::file_manager::{FileKind, FileManager};
use arcfs::fuse_handler;
use std::fs;
use std::io::Write; // Needed for flushing output
use std::path::PathBuf;
// use fuser::{ MountOption, Session }; // Unused but kept for future use

// 1. Define the Command Line Interface (CLI)
#[derive(Parser)]
#[command(name = "ArcFS")]
#[command(about = "A deduplicating, content-addressable filesystem", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(long, default_value = "./my_storage")]
    storage_dir: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Save a file to ArcFS
    Write {
        /// The path to the file you want to upload
        file_path: PathBuf,
    },
    /// Read a file back from ArcFS
    Read {
        /// The name of the file inside ArcFS
        file_name: String,
    },
    /// List all files stored in ArcFS
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
    /// Set/replace tags for an inode
    TagSet {
        /// Target inode id
        inode_id: u64,
        /// Human label used in metadata
        filename: String,
        /// Tags to set (space-separated)
        tags: Vec<String>,
    },
    /// Set/replace tags for a file using a live path like docs/sub/report.txt
    TagSetPath {
        /// Path relative to ArcFS root (without mount prefix)
        path: String,
        /// Tags to set (space-separated)
        tags: Vec<String>,
    },
    /// Get tags for an inode
    TagGet {
        /// Target inode id
        inode_id: u64,
    },
    /// Query inodes by tag intersection
    TagQuery {
        /// Tags to query with AND semantics
        tags: Vec<String>,
    },
    /// List possible next tags from a partial query
    TagNext {
        /// Current query tag path
        tags: Vec<String>,
    },
    /// Delete all tags for an inode
    TagDelete {
        /// Target inode id
        inode_id: u64,
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
    let storage_path = args.storage_dir;
    let manager = FileManager::new(storage_path.to_str().unwrap());

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

            // 2. Ingest it into ArcFS
            println!("Writing '{}' ({} bytes)...", filename, data.len());
            match manager.write_file(filename, &data) {
                Ok(_) => println!("Success! Saved as '{}'", filename),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Read { file_name } => {
            // 1. Ask ArcFS for the bytes
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
                println!("Files in ArcFS:");
                for file in files {
                    println!(" - {}", file);
                }
            }
        }
        Commands::Mount { mount_point } => {
            println!("Mounting ArcFS to {}...", mount_point);
            println!("(Press Ctrl+C to unmount)");

            // Ensure the mount point exists
            fs::create_dir_all(&mount_point).unwrap();

            // Start the FUSE Driver
            let options = vec![
                fuser::MountOption::RW, // Read-Only
                fuser::MountOption::AllowOther,
                fuser::MountOption::FSName("arcfs".to_string()),
                fuser::MountOption::AutoUnmount // Helps clean up on exit
            ];

            let fs_impl = fuse_handler::ArcFS::new(manager);

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
        Commands::TagSet {
            inode_id,
            filename,
            tags,
        } => {
            if tags.is_empty() {
                eprintln!("Error: provide at least one tag");
                return;
            }

            match manager.set_file_tags(inode_id, &filename, tags) {
                Ok(_) => println!("Tags updated for inode {}", inode_id),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::TagSetPath { path, tags } => {
            if tags.is_empty() {
                eprintln!("Error: provide at least one tag");
                return;
            }

            match manager.set_file_tags_by_path(&path, tags) {
                Ok(inode_id) => println!("Tags updated for path '{}' (inode {})", path, inode_id),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::TagGet { inode_id } => match manager.get_file_tags(inode_id) {
            Ok(tags) => {
                if tags.is_empty() {
                    println!("No tags for inode {}", inode_id);
                } else {
                    println!("inode {} tags: {}", inode_id, tags.join(", "));
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        },
        Commands::TagQuery { tags } => {
            if tags.is_empty() {
                eprintln!("Error: provide at least one tag");
                return;
            }

            match manager.get_files_by_tags(&tags) {
                Ok(inodes) => {
                    if inodes.is_empty() {
                        println!("No inodes matched");
                    } else {
                        println!("Matched inodes: {:?}", inodes);
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::TagNext { tags } => match manager.get_next_level_tags(&tags) {
            Ok(next) => {
                if next.is_empty() {
                    println!("No next-level tags");
                } else {
                    println!("Next tags: {}", next.join(", "));
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        },
        Commands::TagDelete { inode_id } => match manager.delete_file_tags(inode_id) {
            Ok(_) => println!("Deleted tags for inode {}", inode_id),
            Err(e) => eprintln!("Error: {}", e),
        },
    }
}
