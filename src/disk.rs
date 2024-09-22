// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2023
// - Volker Schwaberow <volker@schwaberow.de>

use std::fmt::Debug;
use std::fs::File;
use std::io::{self, Error, ErrorKind, Read, Result, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::ZipArchive;

pub const ADF_SECTOR_SIZE: usize = 512;
pub const ADF_TRACK_SIZE: usize = 11 * ADF_SECTOR_SIZE;
pub const ADF_NUM_TRACKS: usize = 80 * 2;
pub const ROOT_BLOCK: usize = 880;

#[derive(Debug, Clone)]
pub struct ADF {
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
pub enum DiskType {
    OFS,
    FFS,
}

#[derive(Debug)]
pub struct FileInfo {
    pub name: String,
    pub size: u32,
    pub is_dir: bool,
    pub protection: u32,
    pub creation_date: SystemTime,
}

#[derive(Debug)]
pub struct DiskInfo {
    pub filesystem: String,
    pub disk_name: String,
    pub creation_date: u32,
    pub disk_size: u32,
    pub heads: u8,
    pub tracks: u8,
    pub sectors_per_track: u8,
    pub bytes_per_sector: u16,
    pub hash_table_size: u32,
    pub first_reserved_block: u32,
    pub last_reserved_block: u32,
}

impl DiskInfo {
    pub fn as_string(&self) -> String {
        format!("{:?}", self)
    }
}

impl std::fmt::Display for DiskInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub struct ExtractedFile {
    name: String,
    size: u32,
    header_block: u32,
    is_ascii: bool,
    contents: Vec<u8>,
}

impl ExtractedFile {
    pub fn as_string(&self) -> io::Result<String> {
        if self.is_ascii {
            Ok(String::from_utf8(self.contents.clone())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "File contents are not ASCII",
            ))
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.contents
    }
}

pub fn load_adf_from_zip(zip_data: &[u8], adf_filename: &str) -> io::Result<ADF> {
    let reader = std::io::Cursor::new(zip_data);
    let mut archive =
        ZipArchive::new(reader).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        if file.name() == adf_filename {
            let mut adf_data = Vec::new();
            file.read_to_end(&mut adf_data)?;
            return ADF::from_bytes(&adf_data);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "ADF file not found in ZIP archive",
    ))
}

impl ADF {
    pub fn format(&mut self, disk_type: DiskType, disk_name: &str) -> Result<()> {
        self.data.fill(0);
        self.write_boot_block(disk_type)?;
        self.write_root_block(disk_type, disk_name)?;
        self.write_bitmap_blocks()?;
        Ok(())
    }
    pub fn extract_file(&self, file_name: &str) -> io::Result<ExtractedFile> {
        let root_files = self.list_root_directory()?;

        for file_info in root_files {
            if file_info.name == file_name {
                if file_info.is_dir {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Cannot extract a directory",
                    ));
                }

                let file_header_block = self.find_file_header_block(ROOT_BLOCK, file_name)?;
                let contents = self.read_file_contents(file_header_block)?;
                let is_ascii = contents
                    .iter()
                    .all(|&b| b.is_ascii_graphic() || b.is_ascii_whitespace());

                return Ok(ExtractedFile {
                    name: file_name.to_string(),
                    size: file_info.size as u32,
                    header_block: file_header_block as u32,
                    is_ascii,
                    contents,
                });
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File '{}' not found", file_name),
        ))
    }

    fn find_file_header_block(&self, dir_block: usize, file_name: &str) -> io::Result<usize> {
        let block_data = self.read_sector(dir_block);

        for i in (24..=51).rev() {
            let sector = u32::from_be_bytes([
                block_data[i * 4],
                block_data[i * 4 + 1],
                block_data[i * 4 + 2],
                block_data[i * 4 + 3],
            ]);
            if sector != 0 {
                let file_info = self.read_file_header(sector as usize)?;
                if file_info.name == file_name {
                    return Ok(sector as usize);
                }
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File header block for '{}' not found", file_name),
        ))
    }

    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        if data.len() != ADF_TRACK_SIZE * ADF_NUM_TRACKS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid ADF size: expected {} bytes, got {} bytes",
                    ADF_TRACK_SIZE * ADF_NUM_TRACKS,
                    data.len()
                ),
            ));
        }
        Ok(ADF {
            data: data.to_vec(),
        })
    }

    pub fn from_file(path: &str) -> Result<ADF> {
        let mut file = File::open(path)?;
        let mut data = vec![0; ADF_TRACK_SIZE * ADF_NUM_TRACKS];
        file.read_exact(&mut data)?;
        Ok(ADF { data })
    }

    pub fn write_to_file(&self, path: &str) -> Result<()> {
        let mut file = File::create(path)?;
        file.write_all(&self.data)?;
        Ok(())
    }

    pub fn read_sector(&self, sector: usize) -> &[u8] {
        let offset = sector * ADF_SECTOR_SIZE;
        &self.data[offset..offset + ADF_SECTOR_SIZE]
    }

    pub fn write_sector(&mut self, sector: usize, data: &[u8]) -> Result<()> {
        if data.len() != ADF_SECTOR_SIZE {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Invalid sector data size",
            ));
        }
        let offset = sector * ADF_SECTOR_SIZE;
        self.data[offset..offset + ADF_SECTOR_SIZE].copy_from_slice(data);
        Ok(())
    }

    pub fn read_boot_block(&self) -> &[u8] {
        &self.data[0..2 * ADF_SECTOR_SIZE]
    }

    pub fn read_root_block(&self) -> &[u8] {
        self.read_sector(ROOT_BLOCK)
    }

    pub fn list_root_directory(&self) -> Result<Vec<FileInfo>> {
        self.list_directory(ROOT_BLOCK).collect()
    }

    pub fn list_directory(&self, block: usize) -> impl Iterator<Item = Result<FileInfo>> + '_ {
        let block_data = self.read_sector(block);
        (24..=51).rev().filter_map(move |i| {
            let sector = u32::from_be_bytes([
                block_data[i * 4],
                block_data[i * 4 + 1],
                block_data[i * 4 + 2],
                block_data[i * 4 + 3],
            ]);
            if sector != 0 {
                Some(self.read_file_header(sector as usize))
            } else {
                None
            }
        })
    }

    fn read_file_header(&self, block: usize) -> Result<FileInfo> {
        let block_data = self.read_sector(block);

        let name_len = block_data[432] as usize;
        let name = String::from_utf8_lossy(&block_data[433..433 + name_len]).to_string();

        let size = u32::from_be_bytes([block_data[4], block_data[5], block_data[6], block_data[7]]);
        let is_dir = block_data[0] == 2;
        let protection = u32::from_be_bytes([
            block_data[436],
            block_data[437],
            block_data[438],
            block_data[439],
        ]);

        let days = u32::from_be_bytes([
            block_data[440],
            block_data[441],
            block_data[442],
            block_data[443],
        ]);
        let mins = u32::from_be_bytes([
            block_data[444],
            block_data[445],
            block_data[446],
            block_data[447],
        ]);
        let ticks = u32::from_be_bytes([
            block_data[448],
            block_data[449],
            block_data[450],
            block_data[451],
        ]);

        let creation_date = match days
            .checked_mul(86400)
            .and_then(|d| d.checked_add(mins.checked_mul(60).unwrap_or(0)))
            .and_then(|t| t.checked_add(ticks.checked_div(50).unwrap_or(0)))
        {
            Some(secs) => SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs as u64),
            None => SystemTime::UNIX_EPOCH,
        };

        Ok(FileInfo {
            name,
            size,
            is_dir,
            protection,
            creation_date,
        })
    }

    pub fn format_protection_flags(&self, flags: u32) -> String {
        let mut result = String::with_capacity(8);
        result.push(if flags & 0x80 == 0 { 'h' } else { '-' }); // hidden
        result.push(if flags & 0x40 == 0 { 's' } else { '-' }); // script
        result.push(if flags & 0x20 == 0 { 'p' } else { '-' }); // pure
        result.push(if flags & 0x10 == 0 { 'a' } else { '-' }); // archive
        result.push(if flags & 0x08 == 0 { 'r' } else { '-' }); // read
        result.push(if flags & 0x04 == 0 { 'w' } else { '-' }); // write
        result.push(if flags & 0x02 == 0 { 'e' } else { '-' }); // execute
        result.push(if flags & 0x01 == 0 { 'd' } else { '-' }); // delete
        result
    }

    pub fn format_creation_date(time: SystemTime) -> String {
        time.duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "Invalid date".to_string())
    }

    pub fn read_file_contents(&self, block: usize) -> io::Result<Vec<u8>> {
        let block_data = self.read_sector(block);

        match block_data[0] {
            2 => {
                let file_size = u32::from_be_bytes([
                    block_data[4],
                    block_data[5],
                    block_data[6],
                    block_data[7],
                ]) as usize;
                let mut contents = Vec::with_capacity(file_size);

                let mut current_block = u32::from_be_bytes([
                    block_data[16],
                    block_data[17],
                    block_data[18],
                    block_data[19],
                ]) as usize;

                while current_block != 0 && contents.len() < file_size {
                    let data_block = self.read_sector(current_block);
                    let data_size = std::cmp::min(512 - 24, file_size - contents.len());
                    contents.extend_from_slice(&data_block[24..24 + data_size]);
                    current_block = u32::from_be_bytes([
                        data_block[0],
                        data_block[1],
                        data_block[2],
                        data_block[3],
                    ]) as usize;
                }

                if contents.len() != file_size {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        format!(
                            "File size mismatch. Expected: {}, Read: {}",
                            file_size,
                            contents.len()
                        ),
                    ));
                }

                Ok(contents)
            }
            0 => {
                let file_size = u32::from_be_bytes([
                    block_data[4],
                    block_data[5],
                    block_data[6],
                    block_data[7],
                ]) as usize;
                let mut contents = Vec::with_capacity(file_size);
                contents.extend_from_slice(&block_data[24..]);

                let mut current_block = u32::from_be_bytes([
                    block_data[16],
                    block_data[17],
                    block_data[18],
                    block_data[19],
                ]) as usize;
                while current_block != 0 && contents.len() < file_size {
                    let data_block = self.read_sector(current_block);
                    let data_size = std::cmp::min(512, file_size - contents.len());
                    contents.extend_from_slice(&data_block[..data_size]);
                    current_block = u32::from_be_bytes([
                        data_block[0],
                        data_block[1],
                        data_block[2],
                        data_block[3],
                    ]) as usize;
                }

                Ok(contents)
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unexpected block type: {}", block_data[0]),
            )),
        }
    }

    fn write_boot_block(&mut self, disk_type: DiskType) -> Result<()> {
        let mut boot_block = [0u8; ADF_SECTOR_SIZE * 2];

        boot_block[..4].copy_from_slice(b"DOS\0");

        boot_block[3] = match disk_type {
            DiskType::OFS => 0,
            DiskType::FFS => 1,
        };

        self.data[..ADF_SECTOR_SIZE * 2].copy_from_slice(&boot_block);
        Ok(())
    }

    fn write_root_block(&mut self, disk_type: DiskType, disk_name: &str) -> Result<()> {
        let mut root_block = [0u8; ADF_SECTOR_SIZE];

        root_block[0] = 2;

        root_block[ADF_SECTOR_SIZE - 4] = match disk_type {
            DiskType::OFS => 0,
            DiskType::FFS => 1,
        };

        root_block[12..14].copy_from_slice(&72u16.to_be_bytes());

        if matches!(disk_type, DiskType::FFS) {
            root_block[ADF_SECTOR_SIZE - 200] = 0xFF;
            for i in 0..25 {
                let block_num = u32::to_be_bytes(ROOT_BLOCK as u32 + 1 + i as u32);
                root_block[ADF_SECTOR_SIZE - 196 + i * 4..ADF_SECTOR_SIZE - 192 + i * 4]
                    .copy_from_slice(&block_num);
            }
        }

        let name_bytes = disk_name.as_bytes();
        let name_len = std::cmp::min(name_bytes.len(), 30);
        root_block[ADF_SECTOR_SIZE - 80] = name_len as u8;
        root_block[ADF_SECTOR_SIZE - 79..ADF_SECTOR_SIZE - 79 + name_len]
            .copy_from_slice(&name_bytes[..name_len]);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let days = u32::to_be_bytes((now.as_secs() / 86400) as u32);
        let mins = u32::to_be_bytes(((now.as_secs() % 86400) / 60) as u32);
        let ticks = u32::to_be_bytes(((now.as_secs() % 60) * 50) as u32);

        root_block[ADF_SECTOR_SIZE - 92..ADF_SECTOR_SIZE - 88].copy_from_slice(&days);
        root_block[ADF_SECTOR_SIZE - 88..ADF_SECTOR_SIZE - 84].copy_from_slice(&mins);
        root_block[ADF_SECTOR_SIZE - 84..ADF_SECTOR_SIZE - 80].copy_from_slice(&ticks);

        self.write_sector(ROOT_BLOCK, &root_block)
    }

    fn write_bitmap_blocks(&mut self) -> Result<()> {
        let mut bitmap_block = [0xFFu8; ADF_SECTOR_SIZE];

        bitmap_block[0] = 0xF8;
        bitmap_block[1] = 0xFF;
        bitmap_block[2] = 0xFF;

        self.write_sector(ROOT_BLOCK + 1, &bitmap_block)?;
        self.write_sector(ROOT_BLOCK + 2, &[0xFFu8; ADF_SECTOR_SIZE])?;

        Ok(())
    }
    pub fn information(&self) -> io::Result<DiskInfo> {
        let root_block = self.read_sector(ROOT_BLOCK);
        Ok(DiskInfo {
            filesystem: if root_block[3] & 1 == 1 {
                "FFS".to_string()
            } else {
                "OFS".to_string()
            },
            disk_name: self.read_disk_name()?,
            creation_date: u32::from_be_bytes([
                root_block[16],
                root_block[17],
                root_block[18],
                root_block[19],
            ]) as u32,
            disk_size: (ADF_TRACK_SIZE * ADF_NUM_TRACKS) as u32,
            heads: 2,
            tracks: (ADF_NUM_TRACKS / 2) as u8,
            sectors_per_track: 11,
            bytes_per_sector: 512,
            hash_table_size: u32::from_be_bytes([
                root_block[12],
                root_block[13],
                root_block[14],
                root_block[15],
            ]),
            first_reserved_block: u32::from_be_bytes([
                root_block[128],
                root_block[129],
                root_block[130],
                root_block[131],
            ]),
            last_reserved_block: u32::from_be_bytes([
                root_block[132],
                root_block[133],
                root_block[134],
                root_block[135],
            ]),
        })
    }

    pub fn list(&self) -> Result<String> {
        let mut output = String::new();

        let files = self.list_root_directory()?;

        for file in files {
            output.push_str(&format!("{} ({} bytes)\n", file.name, file.size));
        }

        Ok(output)
    }

    fn read_disk_name(&self) -> io::Result<String> {
        let root_block = self.read_sector(ROOT_BLOCK);
        let name_len = root_block[ADF_SECTOR_SIZE - 80] as usize;
        let name = String::from_utf8_lossy(
            &root_block[ADF_SECTOR_SIZE - 79..ADF_SECTOR_SIZE - 79 + name_len],
        )
        .to_string();
        Ok(name)
    }
}
