// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2023
// - Volker Schwaberow <volker@schwaberow.de>

use adflib::disk::{DiskType, ADF, ADF_NUM_TRACKS, ADF_TRACK_SIZE};
use clap::{Arg, Command};
use std::fs::File;
use std::io::Write;
use std::fs;
use std::path::Path;

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
            Command::new("create")
                .about("Creates a new empty ADF file")
                .arg(
                    Arg::new("FILE")
                        .required(true)
                        .help("The ADF file to create"),
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

            let files = adf.list_directory(directory)?;
            println!("Directory of {}", file_path);
            println!("Name                 Size    Flags");
            println!("----                 ----    -----");
            for file in &files {
                let flags = if file.is_dir { "  d" } else { "---" };
                println!("{:<20} {:>5}  {}", file.name, file.size, flags);
            }
            println!("{} files", files.len());
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
                    file.write_all(&contents)?;
                }
                None => {
                    std::io::stdout().write_all(&contents)?;
                }
            }
        }
        Some(("info", sub_matches)) => {
            let file_path = sub_matches.get_one::<String>("FILE").unwrap();
            let adf = ADF::from_file(file_path)?;
            let info = adf.information()?;
            println!("ADF Information for {}:\n{}", file_path, info);
        }
        Some(("create", sub_matches)) => {
            let file_path = sub_matches.get_one::<String>("FILE").unwrap();
            let adf = ADF {
                data: vec![0; ADF_TRACK_SIZE * ADF_NUM_TRACKS],
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
