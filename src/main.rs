// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2023
// - Volker Schwaberow <volker@schwaberow.de>

use adflib::disk::{
    ADF, ADF_NUM_SECTORS, ADF_NUM_TRACKS, ADF_TRACK_SIZE, BitmapInfo, DiskInfo, DiskType, FileInfo,
};
use adflib::dms::{convert_dms_to_adf, DMSInfo, DMSReader};
use chrono::{DateTime, Utc};
use clap::{ArgAction, CommandFactory, Parser, Subcommand};
use std::fs::File;
use std::io::{self, Write};
use std::time::UNIX_EPOCH;

#[derive(Parser)]
#[command(
    name = "adflibtool",
    version = env!("CARGO_PKG_VERSION"),
    author = "Volker Schwaberow <volker@schwaberow.de>",
    about = "Enhanced ADF/DMS toolkit"
)]
struct Cli {
    #[arg(short, long, action = ArgAction::SetTrue)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    List {
        file: String,
        #[arg(short, long, default_value = "880")]
        directory: usize,
    },
    Extract {
        adf_file: String,
        file_name: String,
        #[arg(short, long)]
        output: Option<String>,
    },
    Info {
        file: String,
    },
    Dms {
        #[command(subcommand)]
        subcmd: DmsCommands,
    },
    Bitmap {
        #[command(subcommand)]
        subcmd: BitmapCommands,
    },
    Format {
        file: String,
        #[arg(short, long, default_value = "OFS")]
        disk_type: String,
        #[arg(short, long, default_value = "Untitled")]
        name: String,
    },
    Create {
        file: String,
    },
    Dir {
        #[command(subcommand)]
        subcmd: DirCommands,
    },
    Block {
        #[command(subcommand)]
        subcmd: BlockCommands,
    },
    Dump {
        file: String,
        #[arg(short, long)]
        sector: Option<usize>,
    },
}

#[derive(Subcommand)]
enum DmsCommands {
    Info {
        file: String,
    },
    Convert {
        input: String,
        output: String,
    },
}

#[derive(Subcommand)]
enum BitmapCommands {
    Info {
        file: String,
        #[arg(short, long, action = ArgAction::SetTrue)]
        full: bool,
    },
    Set {
        file: String,
        block: usize,
        status: bool,
    },
    Defrag {
        file: String,
    },
}

#[derive(Subcommand)]
enum DirCommands {
    Mkdir {
        file: String,
        dir_path: String,
    },
    Rmdir {
        file: String,
        dir_path: String,
    },
    Rename {
        file: String,
        old_path: String,
        new_name: String,
    },
}

#[derive(Subcommand)]
enum BlockCommands {
    Status {
        file: String,
        block: usize,
    },
    Allocate {
        file: String,
    },
    Free {
        file: String,
        block: usize,
    },
    Fragmentation {
        file: String,
    },
}

fn print_dms_info(info: &DMSInfo, file_path: &str) {
    let datetime = DateTime::<Utc>::from_timestamp(info.date as i64, 0);
    let formatted_date = datetime.map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()).unwrap_or_else(|| "Invalid date".to_string());
    println!("DMS Information for: {}", file_path);
    println!("------------------------");
    println!("Signature: {}", info.signature);
    println!("Header Type: {}", info.header_type);
    println!("Info Bits: {}", info.info_bits);
    println!("Date: {}", formatted_date);
    println!("Compression: {}", info.compression_mode);
}

fn print_disk_info(info: &DiskInfo, file_path: &str) {
    println!("ADF Information for: {}", file_path);
    println!("------------------------");
    println!("Filesystem:      {}", info.filesystem);
    println!("Disk Name:       {}", info.disk_name);
    let creation_date = DateTime::<Utc>::from_timestamp(info.creation_date as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "Invalid date".to_string());
    println!("Creation Date:   {}", creation_date);
    println!("Disk Size:       {} bytes", info.disk_size);
    println!("Geometry:");
    println!("  Heads:         {}", info.heads);
    println!("  Tracks:        {}", info.tracks);
    println!("  Sectors/Track: {}", info.sectors_per_track);
    println!("  Bytes/Sector:  {}", info.bytes_per_sector);
    println!("Hash Table Size: {}", info.hash_table_size);
    println!(
        "Reserved Blocks: {} - {}",
        info.first_reserved_block, info.last_reserved_block
    );
}

fn display_bitmap_info(info: &BitmapInfo, full: bool) {
    println!("Bitmap size: {} blocks", info.total_blocks);
    println!("Free blocks: {}", info.free_blocks);
    println!("Used blocks: {}", info.used_blocks);
    println!("Disk usage:  {:.2}%", info.disk_usage_percentage);
    if full {
        println!("\nBlock allocation map:");
        for (i, &is_used) in info.block_allocation_map.iter().enumerate() {
            print!("{}", if is_used { '#' } else { '.' });
            if (i + 1) % 44 == 0 {
                println!();
            }
        }
        println!("\n. = Free block, # = Used block");
    }
}

fn print_directory_listing(file_path: &str, files: &[FileInfo]) {
    println!("Directory listing for {}", file_path);
    println!(
        "{:<20} {:>8}   {:<6}    {}",
        "Name", "Size", "Flags", "Creation Date"
    );
    println!("{:-<60}", "");
    for file in files {
        let flags = if file.is_dir { "dir" } else { "file" };
        let date = file
            .creation_date
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        println!("{:<20} {:>8}   {:<6}    {}", file.name, file.size, flags, date);
    }
    println!("\nTotal entries: {}", files.len());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            Cli::command().print_help()?;
            return Ok(());
        }
    };

    match command {
        Commands::List { file, directory } => {
            if file.to_lowercase().ends_with(".dms") {
                let file_path = file.clone();
                let file_handle = File::open(&file_path)?;
                let mut reader = io::BufReader::new(file_handle);
                let dms_reader = DMSReader::new(&mut reader)?;
                let dms_info = dms_reader.info();
                print_dms_info(&dms_info, &file_path);
            } else {
                let adf = ADF::from_file(&file)?;
                let files = adf
                    .list_directory(directory)
                    .collect::<Result<Vec<FileInfo>, _>>()?;
                print_directory_listing(&file, &files);
            }
        }
        Commands::Extract {
            adf_file,
            file_name,
            output,
        } => {
            let adf = ADF::from_file(&adf_file)?;
            let extracted = adf.extract_file(&file_name)?;
            match output {
                Some(path) => {
                    let mut out_file = File::create(path)?;
                    out_file.write_all(extracted.as_bytes())?;
                }
                None => {
                    io::stdout().write_all(extracted.as_bytes())?;
                }
            }
        }
        Commands::Info { file } => {
            let adf = ADF::from_file(&file)?;
            let info = adf.information()?;
            print_disk_info(&info, &file);
        }
        Commands::Dms { subcmd } => match subcmd {
            DmsCommands::Info { file } => {
                let file_path = file.clone();
                let file_handle = File::open(&file_path)?;
                let mut reader = io::BufReader::new(file_handle);
                let dms_reader = DMSReader::new(&mut reader)?;
                let dms_info = dms_reader.info();
                print_dms_info(&dms_info, &file_path);
            }
            DmsCommands::Convert { input, output } => {
                convert_dms_to_adf(&input, &output)?;
                println!("Successfully converted {} to {}", input, output);
            }
        },
        Commands::Bitmap { subcmd } => match subcmd {
            BitmapCommands::Info { file, full } => {
                let adf = ADF::from_file(&file)?;
                let bitmap_info = adf.get_bitmap_info();
                println!("Bitmap information for {}:", file);
                display_bitmap_info(&bitmap_info, full);
            }
            BitmapCommands::Set { file, block, status } => {
                let mut adf = ADF::from_file(&file)?;
                adf.set_block_status(block, status)?;
                adf.write_to_file(&file)?;
                println!("Block {} set to {}", block, status);
            }
            BitmapCommands::Defrag { file } => {
                let mut adf = ADF::from_file(&file)?;
                adf.defragment()?;
                adf.write_to_file(&file)?;
                println!("ADF file defragmented");
            }
        },
        Commands::Format {
            file,
            disk_type,
            name,
        } => {
            let disk_type = match disk_type.as_str() {
                "OFS" => DiskType::OFS,
                "FFS" => DiskType::FFS,
                _ => return Err("Invalid disk type provided".into()),
            };
            let mut adf = if let Ok(existing) = ADF::from_file(&file) {
                existing
            } else {
                ADF {
                    data: vec![0; ADF_TRACK_SIZE * ADF_NUM_TRACKS],
                    bitmap: vec![false; ADF_NUM_SECTORS],
                }
            };
            adf.format(disk_type, &name)?;
            adf.write_to_file(&file)?;
            println!(
                "Formatted ADF file: {} (Type: {:?}, Name: {})",
                file, disk_type, name
            );
        }
        Commands::Create { file } => {
            let adf = ADF {
                data: vec![0; ADF_TRACK_SIZE * ADF_NUM_TRACKS],
                bitmap: vec![false; ADF_NUM_SECTORS],
            };
            adf.write_to_file(&file)?;
            println!("Created empty ADF file: {}", file);
        }
        Commands::Dir { subcmd } => match subcmd {
            DirCommands::Mkdir { file, dir_path } => {
                let mut adf = ADF::from_file(&file)?;
                adf.create_directory(&dir_path)?;
                adf.write_to_file(&file)?;
                println!("Created directory '{}' in {}", dir_path, file);
            }
            DirCommands::Rmdir { file, dir_path } => {
                let mut adf = ADF::from_file(&file)?;
                adf.delete_directory(&dir_path)?;
                adf.write_to_file(&file)?;
                println!("Removed directory '{}' from {}", dir_path, file);
            }
            DirCommands::Rename {
                file,
                old_path,
                new_name,
            } => {
                let mut adf = ADF::from_file(&file)?;
                adf.rename_directory(&old_path, &new_name)?;
                adf.write_to_file(&file)?;
                println!(
                    "Renamed directory '{}' to '{}' in {}",
                    old_path, new_name, file
                );
            }
        },
        Commands::Block { subcmd } => match subcmd {
            BlockCommands::Status { file, block } => {
                let adf = ADF::from_file(&file)?;
                if let Some(status) = adf.get_block_status(block) {
                    println!(
                        "Block {} is {}",
                        block,
                        if status { "free" } else { "used" }
                    );
                } else {
                    println!("Invalid block index: {}", block);
                }
            }
            BlockCommands::Allocate { file } => {
                let mut adf = ADF::from_file(&file)?;
                match adf.allocate_block() {
                    Ok(idx) => {
                        adf.write_to_file(&file)?;
                        println!("Allocated block: {}", idx);
                    }
                    Err(e) => println!("Allocation failed: {}", e),
                }
            }
            BlockCommands::Free { file, block } => {
                let mut adf = ADF::from_file(&file)?;
                adf.set_block_status(block, true)?;
                adf.write_to_file(&file)?;
                println!("Marked block {} as free", block);
            }
            BlockCommands::Fragmentation { file } => {
                let adf = ADF::from_file(&file)?;
                let score = adf.get_fragmentation_score();
                println!("Fragmentation score (used blocks count): {}", score);
            }
        },
        Commands::Dump { file, sector } => {
            let adf = ADF::from_file(&file)?;
            if let Some(sector) = sector {
                let data = adf.read_sector(sector);
                println!("Sector {} data (hex):", sector);
                for chunk in data.chunks(16) {
                    for b in chunk {
                        print!("{:02X} ", b);
                    }
                    println!();
                }
            } else {
                println!("Boot block:");
                for b in adf.read_boot_block().chunks(16) {
                    for byte in b {
                        print!("{:02X} ", byte);
                    }
                    println!();
                }
                println!("\nRoot block:");
                for b in adf.read_root_block().chunks(16) {
                    for byte in b {
                        print!("{:02X} ", byte);
                    }
                    println!();
                }
                println!("\nBitmap block (sector {}):", 880 + 1);
                for b in adf.read_sector(880 + 1).chunks(16) {
                    for byte in b {
                        print!("{:02X} ", byte);
                    }
                    println!();
                }
            }
        }
    }

    Ok(())
}