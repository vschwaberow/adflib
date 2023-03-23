// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2023
// - Volker Schwaberow <volker@schwaberow.de>

use clap::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cmd = Command::new("adflibtesttool")
        .bin_name("adflibtesttool")
        .version("0.0.1")
        .author("Volker Schwaberow <volker@schwaberow.de>")
        .about("ADFlib test tool")
        .subcommand(Command::new("read").about("Reads an ADF file"));

    let matches = cmd.get_matches();
    let matches = match matches.subcommand() {
        Some(("read", matches)) => matches,
        _ => unreachable!("we will never end here"),
    };
    Ok(())
}
