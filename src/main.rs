// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2023
// - Volker Schwaberow <volker@schwaberow.de>

use adflib::disk::{
    BitmapInfo, DiskInfo, DiskType, FileInfo, ADF, ADF_NUM_SECTORS, ADF_NUM_TRACKS, ADF_TRACK_SIZE,
};
use adflib::dms::{convert_dms_to_adf, DMSInfo, DMSReader};
use chrono::{DateTime, Utc};
use clap::{Arg, Command};
use std::fs::File;
use std::io::Write;
use std::time::UNIX_EPOCH;

fn print_dms_info(info: &DMSInfo, file_path: &str) {
    println!("DMS Information for: {}", file_path);
    println!("------------------------");
    println!("Signature: {}", info.signature);
    println!("Header Type: {}", info.header_type);
    println!("Info bits: {:#010x}", info.info_bits);
    println!("Date: {}", info.date);
    println!("Compression: {}", info.compression_mode);
}

fn print_disk_info(info: &DiskInfo, file_path: &str) {
    println!("ADF Information for: {}", file_path);
    println!("------------------------");
    println!("Filesystem:     {}", info.filesystem);
    println!("Disk Name:      {}", info.disk_name);
    let creation_date = DateTime::<Utc>::from_timestamp(info.creation_date as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "Invalid Date".to_string());

    println!("Creation Date:  {}", creation_date);
    println!("Disk Size:      {} bytes", info.disk_size);
    println!("Geometry:");
    println!("  Heads:        {}", info.heads);
    println!("  Tracks:       {}", info.tracks);
    println!("  Sectors/Track:{}", info.sectors_per_track);
    println!("  Bytes/Sector: {}", info.bytes_per_sector);
    println!("Hash Table Size:{}", info.hash_table_size);
    println!(
        "Reserved Blocks:{} - {}",
        info.first_reserved_block, info.last_reserved_block
    );
}

fn display_bitmap_info(info: &BitmapInfo, full: bool) {
    println!("Bitmap size: {} blocks", info.total_blocks);
    println!("Free blocks: {}", info.free_blocks);
    println!("Used blocks: {}", info.used_blocks);
    println!("Disk usage: {:.2}%", info.disk_usage_percentage);

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
    println!("Directory of {}", file_path);
    println!("Name                 Size    Flags   Creation Date");
    println!("----                 ----    -----   -------------");

    for file in files {
        let flags = if file.is_dir { "  d" } else { "---" };
        let date = file
            .creation_date
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        println!("{:<20} {:>5}  {}     {}", file.name, file.size, flags, date);
    }

    println!("{} files", files.len());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cmd = Command::new("adflibtesttool")
        .bin_name("adflibtesttool")
        .version("0.0.1")
        .author("Volker Schwaberow <volker@schwaberow.de>")
        .about("ADFlib test tool")
        .subcommand(
            Command::new("list")
                .about("Lists contents of an ADF file")
                .arg(Arg::new("FILE").required(true).help("The ADF file to read"))
                .arg(
                    Arg::new("directory")
                        .short('d')
                        .long("directory")
                        .value_name("DIR")
                        .help("Specify a directory to list (default: root)"),
                ),
        )
        .subcommand(
            Command::new("extract")
                .about("Extracts a file from an ADF")
                .arg(
                    Arg::new("ADF_FILE")
                        .required(true)
                        .help("The ADF file to read from"),
                )
                .arg(
                    Arg::new("FILE_NAME")
                        .required(true)
                        .help("The name of the file to extract within the ADF"),
                )
                .arg(
                    Arg::new("OUTPUT")
                        .short('o')
                        .long("output")
                        .value_name("FILE")
                        .help("Output file (default: stdout)"),
                ),
        )
        .subcommand(
            Command::new("bitmap")
                .about("Performs bitmap operations on an ADF file")
                .subcommand(
                    Command::new("info")
                        .about("Displays information about the bitmap of an ADF file")
                        .arg(
                            Arg::new("FILE")
                                .required(true)
                                .help("The ADF file to analyze"),
                        )
                        .arg(
                            Arg::new("full")
                                .short('f')
                                .long("full")
                                .help("Display full block allocation map")
                                .action(clap::ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("set")
                        .about("Sets the status of a block")
                        .arg(
                            Arg::new("FILE")
                                .required(true)
                                .help("The ADF file to modify"),
                        )
                        .arg(
                            Arg::new("BLOCK")
                                .required(true)
                                .help("The block number to set"),
                        )
                        .arg(
                            Arg::new("STATUS")
                                .required(true)
                                .help("The status to set (free/used)"),
                        ),
                )
                .subcommand(
                    Command::new("defragment")
                        .about("Defragments the ADF file")
                        .arg(
                            Arg::new("FILE")
                                .required(true)
                                .help("The ADF file to defragment"),
                        ),
                ),
        )
        .subcommand(
            Command::new("format")
                .about("Formats an ADF file")
                .arg(
                    Arg::new("FILE")
                        .required(true)
                        .help("The ADF file to format"),
                )
                .arg(
                    Arg::new("TYPE")
                        .short('t')
                        .long("type")
                        .value_name("TYPE")
                        .help("Disk type (OFS, FFS)")
                        .default_value("OFS"),
                )
                .arg(
                    Arg::new("NAME")
                        .short('n')
                        .long("name")
                        .value_name("NAME")
                        .help("Disk name")
                        .default_value("Untitled"),
                ),
        )
        .subcommand(
            Command::new("info")
                .about("Displays information about an ADF file")
                .arg(
                    Arg::new("FILE")
                        .required(true)
                        .help("The ADF file to analyze"),
                ),
        )
        .subcommand(
            Command::new("dms")
                .about("Performs operations on DMS files")
                .subcommand(
                    Command::new("info")
                        .about("Displays information about a DMS file")
                        .arg(
                            Arg::new("FILE")
                                .required(true)
                                .help("The DMS file to analyze"),
                        ),
                )
                .subcommand(
                    Command::new("convert")
                        .about("Converts a DMS file to an ADF file")
                        .arg(
                            Arg::new("INPUT")
                                .required(true)
                                .help("The DMS file to convert"),
                        )
                        .arg(
                            Arg::new("OUTPUT")
                                .required(true)
                                .help("The output ADF file"),
                        ),
                ),
        )
        .subcommand(
            Command::new("create")
                .about("Creates a new empty ADF file")
                .arg(
                    Arg::new("FILE")
                        .required(true)
                        .help("The ADF file to create"),
                ),
        )
        .subcommand(
            Command::new("mkdir")
                .about("Creates a new directory in the ADF file")
                .arg(
                    Arg::new("FILE")
                        .required(true)
                        .help("The ADF file to modify"),
                )
                .arg(Arg::new("DIR_PATH").required(true).help(
                    "The path of the directory to create (e.g., 'NewDir' or 'ParentDir/NewDir')",
                )),
        )
        .subcommand(
            Command::new("rmdir")
                .about("Removes a directory from the ADF file")
                .arg(
                    Arg::new("FILE")
                        .required(true)
                        .help("The ADF file to modify"),
                )
                .arg(Arg::new("DIR_PATH").required(true).help(
                    "The path of the directory to remove (e.g., 'OldDir' or 'ParentDir/OldDir')",
                )),
        )
        .subcommand(
            Command::new("rename")
                .about("Renames a directory in the ADF file")
                .arg(
                    Arg::new("FILE")
                        .required(true)
                        .help("The ADF file to modify"),
                )
                .arg(
                    Arg::new("OLD_PATH")
                        .required(true)
                        .help("The current path of the directory"),
                )
                .arg(
                    Arg::new("NEW_NAME")
                        .required(true)
                        .help("The new name for the directory"),
                ),
        );
    let matches = cmd.get_matches();

    match matches.subcommand() {
        Some(("list", sub_matches)) => {
            let file_path = sub_matches.get_one::<String>("FILE").unwrap();
            let adf = ADF::from_file(file_path)?;

            let directory = sub_matches
                .get_one::<String>("directory")
                .map(|s| s.parse::<usize>().unwrap_or(880))
                .unwrap_or(880);

            let files = adf
                .list_directory(directory)
                .collect::<Result<Vec<FileInfo>, _>>()?;
            print_directory_listing(file_path, &files);
        }
        Some(("extract", sub_matches)) => {
            let adf_path = sub_matches.get_one::<String>("ADF_FILE").unwrap();
            let file_name = sub_matches.get_one::<String>("FILE_NAME").unwrap();
            let output_path = sub_matches.get_one::<String>("OUTPUT");

            let adf = ADF::from_file(adf_path)?;
            let contents = adf.extract_file(file_name)?;

            match output_path {
                Some(path) => {
                    let mut file = File::create(path)?;
                    file.write_all(contents.as_bytes())?;
                }
                None => {
                    std::io::stdout().write_all(contents.as_bytes())?;
                }
            }
        }
        Some(("info", sub_matches)) => {
            let file_path = sub_matches.get_one::<String>("FILE").unwrap();
            let adf = ADF::from_file(file_path)?;
            let info = adf.information()?;
            print_disk_info(&info, file_path);
        }
        Some(("dms", dms_matches)) => match dms_matches.subcommand() {
            Some(("info", info_matches)) => {
                let file_path = info_matches.get_one::<String>("FILE").unwrap();
                let file = File::open(file_path)?;
                let mut reader = std::io::BufReader::new(file);
                let dms_reader = DMSReader::new(&mut reader)?;
                let dms_info = dms_reader.info();
                print_dms_info(&dms_info, file_path);
            }
            Some(("convert", convert_matches)) => {
                let input_path = convert_matches.get_one::<String>("INPUT").unwrap();
                let output_path = convert_matches.get_one::<String>("OUTPUT").unwrap();
                convert_dms_to_adf(input_path, output_path)?;
                println!("Successfully converted {} to {}", input_path, output_path);
            }
            _ => unreachable!("Exhaustive subcommand matching should prevent this"),
        },
        Some(("bitmap", sub_matches)) => match sub_matches.subcommand() {
            Some(("info", info_matches)) => {
                let file_path = info_matches.get_one::<String>("FILE").unwrap();
                let full = info_matches.get_flag("full");
                let adf = ADF::from_file(file_path)?;
                let bitmap_info = adf.get_bitmap_info();
                println!("Bitmap information for {}:", file_path);
                display_bitmap_info(&bitmap_info, full);
            }
            Some(("set", set_matches)) => {
                let file_path = set_matches.get_one::<String>("FILE").unwrap();
                let block = set_matches.get_one::<String>("BLOCK").unwrap();
                let status = set_matches.get_one::<String>("STATUS").unwrap();
                let mut adf = ADF::from_file(file_path)?;
                let block_index = block.parse::<usize>()?;
                let status = status.parse::<bool>()?;
                adf.set_block_status(block_index, status)?;
                adf.write_to_file(file_path)?;
                println!("Block {} set to {}", block_index, status);
            }
            Some(("defragment", defragment_matches)) => {
                let file_path = defragment_matches.get_one::<String>("FILE").unwrap();
                let mut adf = ADF::from_file(file_path)?;
                adf.defragment()?;
                adf.write_to_file(file_path)?;
                println!("ADF file defragmented");
            }
            Some(("mkdir", sub_matches)) => {
                let file_path = sub_matches.get_one::<String>("FILE").unwrap();
                let dir_path = sub_matches.get_one::<String>("DIR_PATH").unwrap();
                let mut adf = ADF::from_file(file_path)?;
                adf.create_directory(dir_path)?;
                adf.write_to_file(file_path)?;
                println!(
                    "Created directory '{}' in ADF file: {}",
                    dir_path, file_path
                );
            }
            Some(("rmdir", sub_matches)) => {
                let file_path = sub_matches.get_one::<String>("FILE").unwrap();
                let dir_path = sub_matches.get_one::<String>("DIR_PATH").unwrap();
                let mut adf = ADF::from_file(file_path)?;
                adf.delete_directory(dir_path)?;
                adf.write_to_file(file_path)?;
                println!(
                    "Removed directory '{}' from ADF file: {}",
                    dir_path, file_path
                );
            }
            Some(("rename", sub_matches)) => {
                let file_path = sub_matches.get_one::<String>("FILE").unwrap();
                let old_path = sub_matches.get_one::<String>("OLD_PATH").unwrap();
                let new_name = sub_matches.get_one::<String>("NEW_NAME").unwrap();
                let mut adf = ADF::from_file(file_path)?;
                adf.rename_directory(old_path, new_name)?;
                adf.write_to_file(file_path)?;
                println!(
                    "Renamed directory '{}' to '{}' in ADF file: {}",
                    old_path, new_name, file_path
                );
            }
            _ => unreachable!("Exhaustive subcommand matching should prevent this"),
        },
        Some(("create", sub_matches)) => {
            let file_path = sub_matches.get_one::<String>("FILE").unwrap();
            let adf = ADF {
                data: vec![0; ADF_TRACK_SIZE * ADF_NUM_TRACKS],
                bitmap: vec![false; ADF_NUM_SECTORS],
            };
            adf.write_to_file(file_path)?;
            println!("Created empty ADF file: {}", file_path);
        }
        Some(("format", sub_matches)) => {
            let file_path = sub_matches.get_one::<String>("FILE").unwrap();
            let disk_type_str = sub_matches.get_one::<String>("TYPE").unwrap();
            let disk_name = sub_matches.get_one::<String>("NAME").unwrap();

            let disk_type = match disk_type_str.as_str() {
                "OFS" => DiskType::OFS,
                "FFS" => DiskType::FFS,
                _ => return Err("Invalid disk type".into()),
            };

            let mut adf = if let Ok(existing_adf) = ADF::from_file(file_path) {
                existing_adf
            } else {
                ADF {
                    data: vec![0; ADF_TRACK_SIZE * ADF_NUM_TRACKS],
                    bitmap: vec![false; ADF_NUM_SECTORS],
                }
            };

            adf.format(disk_type, disk_name)?;
            adf.write_to_file(file_path)?;

            println!(
                "Formatted ADF file: {} (Type: {:?}, Name: {})",
                file_path, disk_type, disk_name
            );
        }
        _ => unreachable!("Exhaustive subcommand matching should prevent this"),
    }

    Ok(())
}
