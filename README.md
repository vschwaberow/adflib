# adflib

A Rust library and command-line tool for managing ADF (Amiga Disk File) files.

## Motivation

While working on cross-development for the Amiga, I needed a way to manage ADF files. Existing libraries weren't written in Rust, which was my preferred language for tool development. Thus, I decided to create a Rust-based library for managing ADF files. The library is still in its early stages and is not yet feature-complete or extensively tested.

## Features

- Read and write ADF files
- Extract disk information (filesystem type, disk name, creation date, etc.)
- List files and directories
- Extract files from ADF images
- Add files to ADF images (planned)
- Create new ADF images (planned)

## Library Usage

Add this to your `Cargo.toml`:
```toml
[dependencies]
adflib = "<ACTUAL VERSION>"
```

Basic usage example:

```rust

use adflib::ADF;
use std::io::Result;

fn main() -> Result<()> {
    let adf = ADF::from_file("my_disk.adf")?;
    println!("ADF file loaded successfully");
    Ok(())
}
```

Getting disk information:

```rust
use adflib::ADF;
use std::io::Result;

fn main() -> Result<()> {
    let adf = ADF::from_file("my_disk.adf")?;
    let disk_info = adf.information?;
    println!("ADF file loaded successfully");
    Ok(())
}
```

Extract files from ADF image:

```rust
use adflib::ADF;
use std::io::Result;

fn main() -> Result<()> {
    let adf = ADF::from_file("my_disk.adf")?;
    let extracted_file = adf.extract_file("my_file.txt")?;
    Ok(())
}
```

Read the documentation for more details.

## Command-line Tool

The library comes with a command-line tool for common ADF operations.

### Installation

```bash
cargo install adflib
```

### Usage

```bash
adflib <COMMAND> [OPTIONS]
```
Commands:

`info` Display information about an ADF file
`list` List contents of an ADF file
`extract` Extract files from an ADF image
`create` Create a new ADF image
`bitmap` Show the bitmap of an ADF image

Example:

```bash
adflib info my_disk.adf
```


## Development Status

The library is based on the [ADF File Format](http://lclevy.free.fr/adflib/faq.html) specification and draws inspiration from the concepts in Laurent Clevy's [ADF library](https://github.com/lclevy/ADFlib). However, it's a pure Rust implementation without using any C code or the original ADF library. And it's
rewritten from scratch.

The project is still in active development. Contributions, bug reports, and feature requests are welcome!

## License 

This project is dual-licensed under MIT and Apache 2.0. See the LICENSE-MIT and LICENSE-APACHE files for details.

## Acknowledgments

* [ADF library](https://github.com/lclevy/ADFlib) by Laurent Clevy et al.
* [ADF File Format](http://lclevy.free.fr/adflib/faq.html) specification
* [Rust](https://www.rust-lang.org/)

## Contribution 

Contributions are welcome! If you'd like to contribute, please feel free to submit a pull request or open an issue for discussion. Any help in improving the library, adding features, or enhancing documentation is greatly appreciated.

