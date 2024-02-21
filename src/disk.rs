// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2023
// - Volker Schwaberow <volker@schwaberow.de>

use std::collections::hash_map::Entry;
use std::error::Error;
use std::fs::File;
use std::io::{ErrorKind, Read, Result, Write};
use std::path::Path;

const ADF_SECTOR_SIZE: usize = 512;
const ADF_TRACK_SIZE: usize = 11 * ADF_SECTOR_SIZE;
const ADF_NUM_TRACKS: usize = 80 * 2;
const BOOTBLOCK_SIZE: usize = 1024;

#[derive(Debug)]
pub struct AmigaFile {
    pub header: AmigaFileHeader,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct AmigaFileHeader {
    pub track: u16,
    pub sector: u16,
    pub file_type: u16,
    pub protection_bits: u16,
    pub file_size: u32,
    pub filename: String,
}

#[derive(Debug)]
pub struct DirectoryEntry {
    pub filename: String,
    pub file_type: AmigaFileType,
    pub protection_bits: u8,
    pub file_size: u32,
}

#[derive(Debug)]
pub enum AmigaFileType {
    File,
    Directory,
    Other(u8),
}

#[derive(Debug, Clone)]
pub struct ADF {
    bootblock: Vec<u8>,
    data: Vec<u8>,
}

impl AmigaFile {
    /// Creates a new `AmigaFile` instance with the given `header` and `data`.
    pub fn new(header: AmigaFileHeader, data: Vec<u8>) -> AmigaFile {
        AmigaFile { header, data }
    }

    /// Returns `true` if the `data` field is not empty, and `false` otherwise.
    pub fn is_some(&self) -> bool {
        !self.data.is_empty()
    }

    /// Returns the size of the file in bytes.
    pub fn file_size(&self) -> usize {
        self.data.len()
    }

    /// Sets the size of the file and resizes the data buffer accordingly.
    ///
    /// # Arguments
    ///
    /// * `size` - The new size of the file.
    pub fn set_file_size(&mut self, size: usize) {
        self.data.resize(size, 0);
    }
}

impl ADF {
    /// Returns an `AmigaFile` instance for the file at the given path.
    ///
    /// # Arguments
    ///
    /// * `self` - The `Disk` instance.
    /// * `path` - The path of the file to get.
    ///
    /// # Returns
    ///
    /// A `Result` containing an `AmigaFile` instance if the file is found and its contents and header were read successfully,
    /// or an `std::io::Error` if the file is not found or the path is invalid.
    pub fn get_file(&self, path: &Path) -> Result<AmigaFile> {
        let file = self.find_file(path)?;
        let header = AmigaFileHeader {
            file_type: file.header.file_type,
            file_size: file.header.file_size,
            track: file.header.track,
            sector: file.header.sector,
            protection_bits: file.header.protection_bits,
            filename: path.file_name().unwrap().to_string_lossy().to_string(),
        };
        Ok(AmigaFile::new(header, file.data))
    }
    /// Gets the file type of a file or directory on the disk.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the file or directory.
    ///
    /// # Returns
    ///
    /// Returns `Ok(AmigaFileType)` if the file or directory was found and its type was determined, or an error if the file or directory could not be found or its type could not be determined.
    pub fn get_file_type(&self, path: &Path) -> Result<AmigaFileType> {
        let file = self.find_file(path)?;
        if file.is_some() {
            if file.header.file_type == 0x0000 {
                Ok(AmigaFileType::File)
            } else if file.header.file_type == 0x0002 {
                Ok(AmigaFileType::Directory)
            } else {
                Ok(AmigaFileType::Other(0))
            }
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found",
            ))
        }
    }

    /// Copies a file or directory from one disk to another.
    ///
    /// # Arguments
    ///
    /// * `src_disk` - A mutable reference to the source disk.
    /// * `dst_disk` - A mutable reference to the destination disk.
    /// * `src_path` - The path of the source file or directory.
    /// * `dst_path` - The path of the destination file or directory.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the file or directory was copied successfully, or an error if the file or directory could not be found or copied.
    pub fn copy(
        src_disk: &mut ADF,
        dst_disk: &mut ADF,
        src_path: &Path,
        dst_path: &Path,
    ) -> Result<()> {
        let src_file_type = src_disk.get_file_type(src_path)?;
        if let AmigaFileType::File = src_file_type {
            let mut data = src_disk.read_file(src_path)?;
            dst_disk.create_file(dst_path)?;
            dst_disk.write_file(dst_path, &mut data)?;
        } else if let AmigaFileType::Directory = src_file_type {
            dst_disk.create_directory(dst_path)?;
            // let entries: Vec<Entry<_, _>> = src_disk.list_directory(src_path);
            // for entry in entries {
            //     let mut src_entry_path = src_path.to_path_buf();
            //     src_entry_path.push(entry.name);
            //     let mut dst_entry_path = dst_path.to_path_buf();
            //     dst_entry_path.push(entry.name);
            //     ADF::copy(src_disk, dst_disk, &src_entry_path, &dst_entry_path)?;
            // }
            Ok(())
        } else {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Unsupported file type"));
        }
        Ok(())
    }

    /// Returns the size of the file at the specified track and sector on the disk.
    ///
    /// # Arguments
    ///
    /// * `track` - The track number of the file.
    /// * `sector` - The sector number of the file.
    ///
    /// # Returns
    ///
    /// Returns the size of the file in bytes if successful, or an error if the file could not be found or read.
    pub fn get_file_size(&self, track: usize, sector: usize) -> Result<usize> {
        let mut track_num = track;
        let mut sector_num = sector;
        let mut size = 0;
        loop {
            let sector_data = self.read_sector(track_num, sector_num);
            let next_track = sector_data[0] as usize;
            let next_sector = sector_data[1] as usize;
            let sector_size = sector_data[2] as usize * 512;
            size += sector_size;
            if next_track == 0 && next_sector == 0 {
                break;
            }
            track_num = next_track;
            sector_num = next_sector;
        }
        Ok(size)
    }

    /// Reads the contents of a file at the specified path from the disk and returns a mutable reference to the data.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to the path of the file to be read.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a mutable reference to a vector of bytes representing the contents of the file if successful,
    /// or an error if the file could not be found or read.
    pub fn read_file_mut(&mut self, path: &Path) -> Result<&mut Vec<u8>> {
        let (track, sector) = self.find_file(path)?;
        let mut track_num = track;
        let mut sector_num = sector;
        let mut data = Vec::new();
        loop {
            let sector_data = self.read_sector(track_num, sector_num);
            let next_track = sector_data[0] as usize;
            let next_sector = sector_data[1] as usize;
            let data_bytes = &sector_data[2..];
            data.extend_from_slice(data_bytes);
            if next_track == 0 && next_sector == 0 {
                break;
            }
            track_num = next_track;
            sector_num = next_sector;
        }
        let file_size = self.get_file_size(track, sector)?;
        if data.len() != file_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found",
            ));
        }
        Ok(data)
    }

    /// Reads the contents of a file at the specified path from the disk.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to the path of the file to be read.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of bytes representing the contents of the file if successful,
    /// or an error if the file could not be found or read.
    pub fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        let (track, sector) = self.find_file(path)?;
        let mut data = Vec::new();
        let mut track_num = track;
        let mut sector_num = sector;
        loop {
            let sector_data = self.read_sector(track_num, sector_num);
            let next_track = sector_data[0] as usize;
            let next_sector = sector_data[1] as usize;
            let data_bytes = &sector_data[2..];
            data.extend_from_slice(data_bytes);
            if next_track == 0 && next_sector == 0 {
                break;
            }
            track_num = next_track;
            sector_num = next_sector;
        }
        Ok(data)
    }

    /// Writes the given data to the file at the specified path on the disk.
    ///
    /// If the file does not exist, it will be created. If the file already exists, its contents will be
    /// replaced with the new data.
    ///
    /// Returns an error if the file could not be found or if there was an error writing to the disk.
    pub fn write_file(&mut self, path: &Path, data: &[u8]) -> Result<()> {
        let (track, sector) = self.find_file(path)?;
        let mut track_num = track;
        let mut sector_num = sector;
        let mut data_offset = 0;

        loop {
            let sector_data = self.read_sector(track_num, sector_num);
            let next_track = sector_data[0] as usize;
            let next_sector = sector_data[1] as usize;
            let mut data_bytes = &mut sector_data[2..];
            let data_len = data.len() - data_offset;
            let sector_len = ADF_SECTOR_SIZE - 2;
            let copy_len = std::cmp::min(data_len, sector_len);

            data_bytes[..copy_len].copy_from_slice(&data[data_offset..data_offset + copy_len]);
            data_offset += copy_len;
            if data_offset >= data.len() {
                sector_data[0] = 0;
                sector_data[1] = 0;
                break;
            }
            if next_track == 0 && next_sector == 0 {
                let (new_track, new_sector) = self.allocate_sector(track_num, sector_num)?;
                sector_data[0] = new_track as u8;
                sector_data[1] = new_sector as u8;
                track_num = new_track;
                sector_num = new_sector;
            } else {
                track_num = next_track;
                sector_num = next_sector;
            }
        }

        Ok(())
    }

    pub fn allocate_sector(
        &mut self,
        prev_track: usize,
        prev_sector: usize,
    ) -> Result<(usize, usize)> {
        let mut track_num = prev_track;
        let mut sector_num = prev_sector;
        loop {
            let sector_data = self.read_sector_mut(track_num, sector_num);
            let next_track = sector_data[0] as usize;
            let next_sector = sector_data[1] as usize;
            if next_track == 0 && next_sector == 0 {
                let (new_track, new_sector) = self.find_free_sector()?;
                sector_data[0] = new_track as u8;
                sector_data[1] = new_sector as u8;
                self.clear_sector(new_track, new_sector)?;
                return Ok((new_track, new_sector));
            }
            track_num = next_track;
            sector_num = next_sector;
        }
    }

    fn clear_sector(&mut self, track_num: usize, sector_num: usize) -> Result<()> {
        let sector_data = self.read_sector_mut(track_num, sector_num);
        sector_data[0] = 0;
        sector_data[1] = 0;
        sector_data[2..].fill(0);
        Ok(())
    }

    fn find_free_sector(&self) -> Result<(usize, usize)> {
        for track_num in 0..ADF_NUM_TRACKS {
            for sector_num in 0..11 {
                let sector_data = self.read_sector(track_num, sector_num)?;
                if sector_data[0] == 0 && sector_data[1] == 0 {
                    return Ok((track_num, sector_num));
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No free sectors available",
        ))
    }

    /// Finds a file in the disk image by its path.
    ///
    /// # Arguments
    ///
    /// * `self` - The `Disk` instance.
    /// * `path` - The path of the file to find.
    ///
    /// # Returns
    ///
    /// A `Result` containing an `AmigaFile` struct if the file is found and its contents and header were read successfully,
    /// or an `std::io::Error` if the file is not found or the path is invalid.
    pub fn find_file(&self, path: &Path) -> Result<AmigaFile> {
        let mut track_num = 0;
        let mut sector_num = 0;

        for component in path.components() {
            let name = match component {
                std::path::Component::Normal(name) => name,
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Invalid file path",
                    ))
                }
            };
            let entries = self.read_directory(track_num, sector_num)?;
            let entry = entries
                .iter()
                .find(|entry| entry.filename == name.to_string_lossy().to_string());
            match entry {
                Some(entry) => {
                    if entry.file_type != AmigaFileType::Directory {
                        let mut data = vec![0; entry.file_size as usize];
                        self.read_file_data(
                            entry.track as usize,
                            entry.sector as usize,
                            &mut data,
                        )?;
                        let header = AmigaFileHeader {
                            file_type: entry.file_type as u16,
                            file_size: entry.file_size as u32,
                            // Add more fields as needed
                        };
                        return Ok(AmigaFile { header, data });
                    }
                    track_num = entry.track as usize;
                    sector_num = entry.sector as usize;
                }
                None => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "File not found",
                    ))
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid file path",
        ))
    }

    /// Reads a directory from the specified track and sector and returns a vector of directory entries.
    ///
    /// # Arguments
    ///
    /// * `track` - The track number of the sector containing the directory.
    /// * `sector` - The sector number of the sector containing the directory.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `DirectoryEntry` structs if the operation was successful, or an `AdfError` if an error occurred.
    pub fn read_directory(&self, track: usize, sector: usize) -> Result<Vec<DirectoryEntry>> {
        let sector_data = self.read_sector(track, sector);
        let mut entries = Vec::new();

        for i in 0..(ADF_SECTOR_SIZE / 32) {
            let offset = i * 32;
            let name_bytes = &sector_data[offset..offset + 30];
            let name = String::from_utf8_lossy(name_bytes)
                .trim_end_matches('\0')
                .to_string();
            let file_type_byte = sector_data[offset + 30];
            let file_type = match file_type_byte {
                0 => AmigaFileType::File,
                1 => AmigaFileType::Directory,
                _ => AmigaFileType::Other(file_type_byte),
            };
            let protection_bits =
                u16::from_le_bytes([sector_data[offset + 31], sector_data[offset + 32]]);
            let file_size = u32::from_le_bytes([
                sector_data[offset + 33],
                sector_data[offset + 34],
                sector_data[offset + 35],
                0,
            ]);
            entries.push(DirectoryEntry {
                filename: name,
                file_type,
                protection_bits: protection_bits as u8,
                file_size,
            })
        }
        Ok(entries)
    }

    /// Reads an ADF file from disk and returns an `ADF` struct.
    ///
    /// # Arguments
    ///
    /// * `path` - A string slice that holds the path to the ADF file.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file cannot be opened or read.
    ///
    /// # Examples
    ///
    /// ```
    /// use adflib::disk::ADF;
    ///
    /// let adf = ADF::from_file("/path/to/adf_file.adf").unwrap();
    /// ```
    pub fn from_file(path: &str) -> Result<ADF> {
        let mut file = File::open(path)?;
        let mut data = vec![0; ADF_TRACK_SIZE * ADF_NUM_TRACKS];
        file.read_exact(&mut data)?;

        let mut bootblock = vec![0; BOOTBLOCK_SIZE];
        file.read_exact(&mut bootblock)?;

        Ok(ADF { bootblock, data })
    }

    /// Writes the bootblock and data to a file at the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - A string slice that holds the path of the file to be created.
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to create the file or write to it.
    ///
    /// # Examples
    ///
    /// ```
    /// use adflib::disk::Disk;
    ///
    /// let disk = Disk::new();
    /// disk.write_to_file("disk.adf").unwrap();
    /// ```
    pub fn write_to_file(&self, path: &str) -> Result<()> {
        let mut file = File::create(path)?;
        file.write_all(&self.bootblock)?;
        file.write_all(&self.data)?;
        Ok(())
    }

    /// Reads a sector from the disk.
    ///
    /// # Arguments
    ///
    /// * `track` - The track number of the sector to read.
    /// * `sector` - The sector number of the sector to read.
    ///
    /// # Returns
    ///
    /// A reference to the sector data.
    ///
    /// # Examples
    ///
    /// ```
    /// let disk = Disk::new();
    /// let sector_data = disk.read_sector(0, 1);
    /// ```
    pub fn read_sector(&self, track: usize, sector: usize) -> &[u8] {
        let offset = track * ADF_TRACK_SIZE + sector * ADF_SECTOR_SIZE;
        if track == 0 && sector == 0 {
            &self.bootblock
        } else {
            &self.data[offset..offset + ADF_SECTOR_SIZE]
        }
    }

    /// Writes a sector to the disk image.
    ///
    /// # Arguments
    ///
    /// * `track` - The track number of the sector to write.
    /// * `sector` - The sector number of the sector to write.
    /// * `data` - The data to write to the sector.
    ///
    /// # Errors
    ///
    /// Returns an error if the length of the data is not equal to the sector size.
    pub fn write_sector(&mut self, track: usize, sector: usize, data: &[u8]) -> Result<()> {
        if data.len() != ADF_SECTOR_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid sector data size",
            ));
        }
        let offset = track * ADF_TRACK_SIZE + sector * ADF_SECTOR_SIZE;
        if track == 0 && sector == 0 {
            self.bootblock.copy_from_slice(data);
        } else {
            self.data[offset..offset + ADF_SECTOR_SIZE].copy_from_slice(data);
        }
        Ok(())
    }

    /// Reads a track from the disk.
    ///
    /// # Arguments
    ///
    /// * `track` - The track number to read.
    ///
    /// # Returns
    ///
    /// A slice containing the bytes of the specified track.
    pub fn read_track(&self, track: usize) -> &[u8] {
        let offset = track * ADF_TRACK_SIZE;
        &self.data[offset..offset + ADF_TRACK_SIZE]
    }

    /// Writes the given track data to the disk image.
    ///
    /// # Arguments
    ///
    /// * `track` - The track number to write the data to.
    /// * `data` - The data to write to the track.
    ///
    /// # Errors
    ///
    /// Returns an error if the length of the data is not equal to the size of a track.
    pub fn write_track(&mut self, track: usize, data: &[u8]) -> Result<()> {
        if data.len() != ADF_TRACK_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid track data size",
            ));
        }
        let offset = track * ADF_TRACK_SIZE;
        self.data[offset..offset + ADF_TRACK_SIZE].copy_from_slice(data);
        Ok(())
    }

    fn create_file(&self, dst_path: &Path) -> Result<()> {
        todo!()
    }

    fn create_directory(&self, dst_path: &Path) -> Result<()> {
        todo!()
    }

    fn list_directory(&self, src_path: &Path) -> Result<()> {
        todo!()
    }
}
