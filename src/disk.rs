// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2023
// - Volker Schwaberow <volker@schwaberow.de>

use crate::consts::*;
use crate::error::{AdfError, Result};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::fs::File;
use std::io::{self, Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::ZipArchive;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ADF {
    pub data: Vec<u8>,
    pub bitmap: Vec<bool>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DiskType {
    OFS,
    FFS,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub size: u32,
    pub is_dir: bool,
    pub protection: u32,
    pub creation_date: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedFile {
    name: String,
    size: u32,
    header_block: u32,
    is_ascii: bool,
    contents: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitmapInfo {
    pub total_blocks: u32,
    pub free_blocks: u32,
    pub used_blocks: u32,
    pub disk_usage_percentage: f32,
    pub block_allocation_map: Vec<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ADFMetadata {
    pub disk_info: DiskInfo,
    pub file_list: Vec<FileInfo>,
    pub bitmap_info: BitmapInfo,
}

impl std::fmt::Display for BitmapInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl ExtractedFile {
    pub fn as_string(&self) -> Result<String> {
        if self.is_ascii {
            Ok(String::from_utf8(self.contents.clone())
                .map_err(|e| AdfError::from(io::Error::new(io::ErrorKind::InvalidData, e)))?)
        } else {
            Err(AdfError::from(io::Error::new(
                io::ErrorKind::InvalidData,
                "File contents are not ASCII",
            )))
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.contents
    }
}

pub fn format_creation_date(time: SystemTime) -> String {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "Invalid date".to_string())
}

pub fn load_adf_from_zip(zip_data: &[u8], adf_filename: &str) -> Result<ADF> {
    let reader = std::io::Cursor::new(zip_data);
    let mut archive = ZipArchive::new(reader)
        .map_err(|e| AdfError::from(io::Error::new(io::ErrorKind::Other, e)))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| AdfError::from(io::Error::new(io::ErrorKind::Other, e)))?;
        if file.name() == adf_filename {
            let mut adf_data = Vec::new();
            file.read_to_end(&mut adf_data)?;
            return ADF::from_bytes(&adf_data);
        }
    }

    Err(AdfError::from(io::Error::new(
        io::ErrorKind::NotFound,
        "ADF file not found in ZIP archive",
    )))
}

impl ADF {
    pub fn new(size: usize, block_size: usize) -> Self {
        ADF {
            data: vec![0; size * block_size],
            bitmap: vec![true; size],
        }
    }

    pub fn extract_metadata(&self) -> Result<ADFMetadata> {
        Ok(ADFMetadata {
            disk_info: self.information()?,
            file_list: self.list_root_directory()?,
            bitmap_info: self.get_bitmap_info(),
        })
    }

    pub fn format_disk(&mut self, disk_name: &str) -> Result<()> {
        self.data.fill(0);
        self.write_boot_block(DiskType::OFS)?;
        self.write_root_block(DiskType::OFS, disk_name)?;
        self.write_bitmap_blocks()?;
        Ok(())
    }

    pub fn extract_file(&self, path: &str, file_name: &str) -> Result<ExtractedFile> {
        let root_files = self.list_root_directory()?;

        for file_info in root_files {
            if file_info.name == file_name {
                if file_info.is_dir {
                    return Err(AdfError::InvalidName(
                        "Cannot extract a directory".to_string(),
                    ));
                }

                let file_header_block = self.find_file_header_block(ROOT_BLOCK, file_name)?;
                let contents = self.read_file_contents(file_header_block)?;

                return Ok(ExtractedFile {
                    name: file_name.to_string(),
                    contents: contents.clone(),
                    is_ascii: contents.iter().all(|&b| b < 128),
                    size: file_info.size,
                    header_block: file_header_block as u32,
                });
            }
        }

        Err(AdfError::NotFound(format!(
            "File '{}' not found",
            file_name
        )))
    }

    fn find_file_header_block(&self, dir_block: usize, file_name: &str) -> Result<usize> {
        let block_data = self.read_sector(dir_block);

        for i in (DIR_ENTRY_START_INDEX..=DIR_ENTRY_END_INDEX).rev() {
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

        Err(AdfError::NotFound(format!(
            "File header block for '{}' not found",
            file_name
        )))
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() != ADF_TRACK_SIZE * ADF_NUM_TRACKS {
            return Err(AdfError::InvalidDiskFormat(format!(
                "Invalid ADF size: expected {} bytes, got {} bytes",
                ADF_TRACK_SIZE * ADF_NUM_TRACKS,
                data.len()
            )));
        }
        Ok(ADF {
            data: data.to_vec(),
            bitmap: vec![false; ADF_NUM_SECTORS],
        })
    }

    pub fn from_file(path: &str) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut data = vec![0; ADF_SECTOR_SIZE * ADF_NUM_SECTORS];
        file.read_exact(&mut data)?;
        ADF::from_bytes(&data)
    }

    pub fn get_bitmap(&self) -> &[bool] {
        &self.bitmap
    }

    pub fn get_bitmap_info(&self) -> BitmapInfo {
        let bitmap_block = self.read_sector(ROOT_BLOCK + 1);
        let mut free_blocks = 0;
        let mut used_blocks = 0;
        let mut block_allocation_map = Vec::with_capacity(ADF_NUM_SECTORS);

        for (i, &byte) in bitmap_block.iter().enumerate() {
            if i < 220 {
                for bit in 0..8 {
                    if i * 8 + bit < ADF_NUM_SECTORS {
                        let is_free = byte & (1 << (7 - bit)) != 0;
                        if is_free {
                            free_blocks += 1;
                        } else {
                            used_blocks += 1;
                        }
                        block_allocation_map.push(!is_free);
                    }
                }
            }
        }

        let disk_usage_percentage = (used_blocks as f64 / ADF_NUM_SECTORS as f64) * 100.0;

        BitmapInfo {
            total_blocks: ADF_NUM_SECTORS as u32,
            free_blocks,
            used_blocks,
            disk_usage_percentage: disk_usage_percentage as f32,
            block_allocation_map,
        }
    }

    pub fn get_block_status(&self, block_index: usize) -> Option<bool> {
        if block_index < self.bitmap.len() {
            Some(self.bitmap[block_index])
        } else {
            None
        }
    }

    pub fn set_block_allocated(&mut self, block: usize, status: bool) -> Result<()> {
        if block < self.bitmap.len() {
            self.bitmap[block] = status;
            Ok(())
        } else {
            Err(AdfError::InvalidBlock(
                block,
                "Block index out of range".to_string(),
            ))
        }
    }

    pub fn defragment(&mut self) -> Result<()> {
        let mut free_blocks = Vec::new();
        for (i, &is_free) in self.bitmap.iter().enumerate() {
            if is_free {
                free_blocks.push(i);
            }
        }
        Ok(())
    }

    pub fn get_fragmentation_score(&self) -> usize {
        self.bitmap.iter().filter(|&&b| !b).count()
    }

    pub fn write_to_file(&self, path: &str) -> Result<()> {
        let mut file = File::create(path)?;
        file.write_all(&self.data)?;
        Ok(())
    }

    pub fn find_contiguous_free_blocks(&self, count: usize) -> Option<usize> {
        let mut free_blocks = Vec::new();
        for (i, &is_free) in self.bitmap.iter().enumerate() {
            if is_free {
                free_blocks.push(i);
            }
        }

        for i in 0..free_blocks.len() - count {
            if free_blocks[i + count] - free_blocks[i] == count {
                return Some(free_blocks[i]);
            }
        }

        None
    }

    pub fn read_sector(&self, sector: usize) -> &[u8] {
        let offset = sector * ADF_SECTOR_SIZE;
        &self.data[offset..offset + ADF_SECTOR_SIZE]
    }

    pub fn write_sector(&mut self, sector: usize, data: &[u8]) -> Result<()> {
        if data.len() != ADF_SECTOR_SIZE {
            return Err(AdfError::InvalidBlock(
                sector,
                format!(
                    "Invalid sector data size: expected {} bytes, got {} bytes",
                    ADF_SECTOR_SIZE,
                    data.len()
                ),
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
        self.list_directory(ROOT_BLOCK)
    }

    pub fn list_directory(&self, block: usize) -> Result<Vec<FileInfo>> {
        let block_data = self.read_sector(block);
        let mut files = Vec::new();

        for i in DIR_ENTRIES_START..=DIR_ENTRIES_END {
            let entry_block = u32::from_be_bytes([
                block_data[i * 4],
                block_data[i * 4 + 1],
                block_data[i * 4 + 2],
                block_data[i * 4 + 3],
            ]);
            if entry_block != 0 {
                match self.read_file_header(entry_block as usize) {
                    Ok(file_info) => files.push(file_info),
                    Err(e) => return Err(e),
                }
            }
        }

        Ok(files)
    }

    fn read_file_header(&self, block: usize) -> Result<FileInfo> {
        let block_data = self.read_sector(block);

        let name_len = block_data[FILE_NAME_LEN_OFFSET] as usize;
        let name =
            String::from_utf8_lossy(&block_data[FILE_NAME_OFFSET..FILE_NAME_OFFSET + name_len])
                .to_string();

        let size = u32::from_be_bytes([
            block_data[FILE_SIZE_OFFSET],
            block_data[FILE_SIZE_OFFSET + 1],
            block_data[FILE_SIZE_OFFSET + 2],
            block_data[FILE_SIZE_OFFSET + 3],
        ]);
        let is_dir = block_data[BLOCK_TYPE_OFFSET] == BLOCK_TYPE_DIRECTORY;
        let protection = u32::from_be_bytes([
            block_data[FILE_PROTECTION_OFFSET],
            block_data[FILE_PROTECTION_OFFSET + 1],
            block_data[FILE_PROTECTION_OFFSET + 2],
            block_data[FILE_PROTECTION_OFFSET + 3],
        ]);

        let days = u32::from_be_bytes([
            block_data[FILE_DAYS_OFFSET],
            block_data[FILE_DAYS_OFFSET + 1],
            block_data[FILE_DAYS_OFFSET + 2],
            block_data[FILE_DAYS_OFFSET + 3],
        ]);
        let mins = u32::from_be_bytes([
            block_data[FILE_MINS_OFFSET],
            block_data[FILE_MINS_OFFSET + 1],
            block_data[FILE_MINS_OFFSET + 2],
            block_data[FILE_MINS_OFFSET + 3],
        ]);
        let ticks = u32::from_be_bytes([
            block_data[FILE_TICKS_OFFSET],
            block_data[FILE_TICKS_OFFSET + 1],
            block_data[FILE_TICKS_OFFSET + 2],
            block_data[FILE_TICKS_OFFSET + 3],
        ]);

        let creation_date = match days
            .checked_mul(SECONDS_PER_DAY as u32)
            .and_then(|d| d.checked_add(mins.checked_mul(SECONDS_PER_MINUTE as u32).unwrap_or(0)))
            .and_then(|t| t.checked_add(ticks.checked_div(TICKS_PER_SECOND).unwrap_or(0)))
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
        let masked_flags = flags & PROTECTION_FLAGS_MASK;
        let mut result = String::with_capacity(8);
        result.push(if masked_flags & PROTECTION_FLAG_HIDDEN == 0 {
            'h'
        } else {
            '-'
        });
        result.push(if masked_flags & PROTECTION_FLAG_SCRIPT == 0 {
            's'
        } else {
            '-'
        });
        result.push(if masked_flags & PROTECTION_FLAG_PURE == 0 {
            'p'
        } else {
            '-'
        });
        result.push(if masked_flags & PROTECTION_FLAG_ARCHIVE == 0 {
            'a'
        } else {
            '-'
        });
        result.push(if masked_flags & PROTECTION_FLAG_READ == 0 {
            'r'
        } else {
            '-'
        });
        result.push(if masked_flags & PROTECTION_FLAG_WRITE == 0 {
            'w'
        } else {
            '-'
        });
        result.push(if masked_flags & PROTECTION_FLAG_EXECUTE == 0 {
            'e'
        } else {
            '-'
        });
        result.push(if masked_flags & PROTECTION_FLAG_DELETE == 0 {
            'd'
        } else {
            '-'
        });
        result
    }

    pub fn calculate_checksum(&self, data: &[u8]) -> u32 {
        let mut checksum = 0u32;
        for chunk in data.chunks(4) {
            let word = u32::from_be_bytes([
                chunk[0],
                chunk.get(1).copied().unwrap_or(0),
                chunk.get(2).copied().unwrap_or(0),
                chunk.get(3).copied().unwrap_or(0),
            ]);
            checksum = checksum.wrapping_add(word);
        }
        !checksum
    }

    pub fn allocate_block(&mut self) -> Result<usize> {
        for block in FIRST_DATA_BLOCK..self.bitmap.len() {
            if !self.bitmap[block] {
                self.set_block_allocated(block, true)?;
                return Ok(block);
            }
        }
        Err(AdfError::DiskFull("No free blocks available".to_string()))
    }

    pub fn find_free_block(&self) -> Option<usize> {
        self.bitmap
            .iter()
            .enumerate()
            .skip(2)
            .find(|&(_, &is_free)| is_free)
            .map(|(index, _)| index)
    }

    pub fn read_file_contents(&self, block: usize) -> Result<Vec<u8>> {
        let block_data = self.read_sector(block);

        match block_data[0] {
            BLOCK_TYPE_FILE_HEADER => {
                let file_size = u32::from_be_bytes([
                    block_data[FILE_SIZE_OFFSET],
                    block_data[FILE_SIZE_OFFSET + 1],
                    block_data[FILE_SIZE_OFFSET + 2],
                    block_data[FILE_SIZE_OFFSET + 3],
                ]) as usize;

                let mut contents = Vec::with_capacity(file_size);
                let mut remaining = file_size;
                let mut current_block = block;

                while remaining > 0 {
                    let data_block = self.read_sector(current_block);
                    let next_block = u32::from_be_bytes([
                        data_block[DATA_BLOCK_NEXT_OFFSET],
                        data_block[DATA_BLOCK_NEXT_OFFSET + 1],
                        data_block[DATA_BLOCK_NEXT_OFFSET + 2],
                        data_block[DATA_BLOCK_NEXT_OFFSET + 3],
                    ]) as usize;

                    let data_size = std::cmp::min(DATA_BLOCK_SIZE, remaining);
                    contents.extend_from_slice(
                        &data_block[DATA_BLOCK_DATA_OFFSET..DATA_BLOCK_DATA_OFFSET + data_size],
                    );
                    remaining -= data_size;

                    if remaining > 0 && next_block == 0 {
                        return Err(AdfError::InvalidBlock(
                            current_block,
                            "Unexpected end of file data chain".to_string(),
                        ));
                    }
                    current_block = next_block;
                }

                if contents.len() != file_size {
                    return Err(AdfError::InvalidBlock(
                        block,
                        format!(
                            "File size mismatch. Expected: {}, Read: {}",
                            file_size,
                            contents.len()
                        ),
                    ));
                }

                Ok(contents)
            }
            BLOCK_TYPE_FILE_LIST => {
                let mut contents = Vec::new();
                let mut current_block = block;

                while current_block != 0 {
                    let list_block = self.read_sector(current_block);
                    let next_block = u32::from_be_bytes([
                        list_block[FILE_LIST_NEXT_OFFSET],
                        list_block[FILE_LIST_NEXT_OFFSET + 1],
                        list_block[FILE_LIST_NEXT_OFFSET + 2],
                        list_block[FILE_LIST_NEXT_OFFSET + 3],
                    ]) as usize;

                    for i in 0..FILE_LIST_ENTRIES_PER_BLOCK {
                        let data_block = u32::from_be_bytes([
                            list_block[FILE_LIST_ENTRY_OFFSET + i * 4],
                            list_block[FILE_LIST_ENTRY_OFFSET + i * 4 + 1],
                            list_block[FILE_LIST_ENTRY_OFFSET + i * 4 + 2],
                            list_block[FILE_LIST_ENTRY_OFFSET + i * 4 + 3],
                        ]) as usize;

                        if data_block != 0 {
                            let data = self.read_sector(data_block);
                            contents.extend_from_slice(
                                &data[DATA_BLOCK_DATA_OFFSET
                                    ..DATA_BLOCK_DATA_OFFSET + DATA_BLOCK_SIZE],
                            );
                        }
                    }

                    current_block = next_block;
                }

                Ok(contents)
            }
            _ => Err(AdfError::InvalidBlock(
                block,
                format!("Unexpected block type: {}", block_data[0]),
            )),
        }
    }

    fn write_boot_block(&mut self, disk_type: DiskType) -> Result<()> {
        let mut boot_block = [0u8; SECTOR_SIZE * BOOT_BLOCK_SIZE];

        // Set the DOS signature
        boot_block[0..BOOT_BLOCK_SIGNATURE_SIZE].copy_from_slice(BOOT_BLOCK_SIGNATURE);

        // Set the flags based on the disk type
        match disk_type {
            DiskType::OFS => boot_block[BOOT_BLOCK_FLAGS_OFFSET] = FILESYSTEM_TYPE_OFS,
            DiskType::FFS => boot_block[BOOT_BLOCK_FLAGS_OFFSET] = FILESYSTEM_TYPE_FFS,
        }

        // Write the boot block
        for i in 0..BOOT_BLOCK_SIZE {
            self.write_sector(i, &boot_block[i * SECTOR_SIZE..(i + 1) * SECTOR_SIZE])?;
        }

        Ok(())
    }

    fn write_root_block(&mut self, disk_type: DiskType, disk_name: &str) -> Result<()> {
        let mut root_block = [0u8; SECTOR_SIZE];

        // Set the disk type
        match disk_type {
            DiskType::OFS => root_block[ROOT_BLOCK_DISK_TYPE_OFFSET] = FILESYSTEM_TYPE_OFS,
            DiskType::FFS => root_block[ROOT_BLOCK_DISK_TYPE_OFFSET] = FILESYSTEM_TYPE_FFS,
        }

        // Set the hash table size
        let hash_table_size: u32 = 72;
        root_block[ROOT_BLOCK_HASH_TABLE_SIZE_OFFSET..ROOT_BLOCK_HASH_TABLE_SIZE_OFFSET + 4]
            .copy_from_slice(&hash_table_size.to_be_bytes());

        // Set the bitmap blocks
        for i in 0..ROOT_BLOCK_BITMAP_COUNT {
            let bitmap_block = BITMAP_BLOCK + i;
            root_block[ROOT_BLOCK_BITMAP_POINTERS_OFFSET + i * 4
                ..ROOT_BLOCK_BITMAP_POINTERS_OFFSET + (i + 1) * 4]
                .copy_from_slice(&(bitmap_block as u32).to_be_bytes());
        }

        // Set the disk name
        let name_bytes = disk_name.as_bytes();
        let name_len = std::cmp::min(name_bytes.len(), FILE_NAME_MAX_LEN);
        root_block[ROOT_BLOCK_NAME_LEN_OFFSET] = name_len as u8;
        root_block[ROOT_BLOCK_NAME_OFFSET..ROOT_BLOCK_NAME_OFFSET + name_len]
            .copy_from_slice(&name_bytes[..name_len]);

        // Set the creation date
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let days = ((now / SECONDS_PER_DAY as u64) as u32).to_be_bytes();
        let mins =
            (((now % SECONDS_PER_DAY as u64) / SECONDS_PER_MINUTE as u64) as u32).to_be_bytes();
        let ticks =
            (((now % SECONDS_PER_MINUTE as u64) * TICKS_PER_SECOND as u64) as u32).to_be_bytes();

        root_block[ROOT_BLOCK_DAYS_OFFSET..ROOT_BLOCK_DAYS_OFFSET + 4].copy_from_slice(&days);
        root_block[ROOT_BLOCK_MINS_OFFSET..ROOT_BLOCK_MINS_OFFSET + 4].copy_from_slice(&mins);
        root_block[ROOT_BLOCK_TICKS_OFFSET..ROOT_BLOCK_TICKS_OFFSET + 4].copy_from_slice(&ticks);

        // Write the root block
        self.write_sector(ROOT_BLOCK, &root_block)?;

        Ok(())
    }

    fn write_bitmap_blocks(&mut self) -> Result<()> {
        let mut bitmap_block = [0xFFu8; ADF_SECTOR_SIZE];

        bitmap_block[BITMAP_HEADER_OFFSET] = BITMAP_HEADER_VALUE;
        bitmap_block[BITMAP_FLAG_OFFSET] = 0xFF;
        bitmap_block[BITMAP_VALID_OFFSET] = 0xFF;

        self.write_sector(BITMAP_BLOCK, &bitmap_block)?;
        self.write_sector(BITMAP_BLOCK + 1, &[0xFFu8; ADF_SECTOR_SIZE])?;

        Ok(())
    }
    pub fn information(&self) -> Result<DiskInfo> {
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

    fn read_disk_name(&self) -> Result<String> {
        let root_block = self.read_sector(ROOT_BLOCK);
        let name_len = root_block[ADF_SECTOR_SIZE - 80] as usize;
        let name = String::from_utf8_lossy(
            &root_block[ADF_SECTOR_SIZE - 79..ADF_SECTOR_SIZE - 79 + name_len],
        )
        .to_string();
        Ok(name)
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self)
            .map_err(|e| AdfError::from(io::Error::new(io::ErrorKind::Other, e)))
    }

    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| AdfError::from(io::Error::new(io::ErrorKind::Other, e)))
    }

    pub fn save_to_json_file(&self, path: &str) -> Result<()> {
        let json = self.to_json()?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    pub fn load_from_json_file(path: &str) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Self::from_json(&contents)
    }

    pub fn create_directory(&mut self, path: &str, dir_name: &str) -> Result<()> {
        // Check if the directory name is valid
        if dir_name.is_empty() || dir_name.len() > FILE_NAME_MAX_LEN || dir_name.contains('/') {
            return Err(AdfError::InvalidName(format!(
                "Invalid directory name: '{}'",
                dir_name
            )));
        }

        // Find the parent directory block
        let parent_block = self.find_directory_block(path)?;

        // Check if the directory already exists
        let existing_files = self.list_directory(parent_block)?;
        for file in existing_files {
            if file.name == dir_name {
                return Err(AdfError::AlreadyExists(format!(
                    "Directory '{}' already exists",
                    dir_name
                )));
            }
        }

        // Allocate a new block for the directory
        let dir_block = self.allocate_block()?;

        // Create the directory block
        let mut dir_data = [0u8; SECTOR_SIZE];
        dir_data[BLOCK_TYPE_OFFSET] = BLOCK_TYPE_DIRECTORY;

        // Set the parent block
        dir_data[DIR_PARENT_OFFSET..DIR_PARENT_OFFSET + 4]
            .copy_from_slice(&(parent_block as u32).to_be_bytes());

        // Set the directory name
        let name_bytes = dir_name.as_bytes();
        let name_len = std::cmp::min(name_bytes.len(), FILE_NAME_MAX_LEN);
        dir_data[DIR_NAME_LEN_OFFSET] = name_len as u8;
        dir_data[DIR_NAME_OFFSET..DIR_NAME_OFFSET + name_len]
            .copy_from_slice(&name_bytes[..name_len]);

        // Set the creation date
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let days = ((now / SECONDS_PER_DAY as u64) as u32).to_be_bytes();
        let mins =
            (((now % SECONDS_PER_DAY as u64) / SECONDS_PER_MINUTE as u64) as u32).to_be_bytes();
        let ticks =
            (((now % SECONDS_PER_MINUTE as u64) * TICKS_PER_SECOND as u64) as u32).to_be_bytes();

        dir_data[DIR_DAYS_OFFSET..DIR_DAYS_OFFSET + 4].copy_from_slice(&days);
        dir_data[DIR_MINS_OFFSET..DIR_MINS_OFFSET + 4].copy_from_slice(&mins);
        dir_data[DIR_TICKS_OFFSET..DIR_TICKS_OFFSET + 4].copy_from_slice(&ticks);

        // Calculate the checksum
        let checksum = self.calculate_checksum(&dir_data[DIR_CHECKSUM_OFFSET..]);
        dir_data[DIR_CHECKSUM_LOCATION..DIR_CHECKSUM_LOCATION + 4]
            .copy_from_slice(&checksum.to_be_bytes());

        // Write the directory block
        self.write_sector(dir_block, &dir_data)?;

        // Add the directory to the parent directory
        self.add_entry_to_directory(parent_block, dir_block as u32, dir_name)?;

        // Update the bitmap blocks
        self.update_bitmap_blocks()?;

        Ok(())
    }

    pub fn delete_directory(&mut self, path: &str) -> Result<()> {
        let (parent_path, dir_name) = split_path(path);
        let parent_block = self.find_directory_block(parent_path)?;
        let dir_block = self.find_file_header_block(parent_block, dir_name)?;

        if self.is_directory_empty(dir_block)? {
            self.remove_entry_from_directory(parent_block, dir_name)?;
            self.set_block_allocated(dir_block, false)?;
            self.update_bitmap_blocks()?;
            Ok(())
        } else {
            Err(AdfError::DirectoryNotEmpty(format!(
                "Directory '{}' is not empty",
                dir_name
            )))
        }
    }

    pub fn is_directory_empty(&self, dir_block: usize) -> Result<bool> {
        let files = self.list_directory(dir_block)?;
        Ok(files.is_empty())
    }

    pub fn rename_directory(
        &mut self,
        path: &str,
        old_name: &str,
        new_name: &str,
    ) -> Result<()> {
        // Check if the new name is valid
        if new_name.is_empty() || new_name.len() > FILE_NAME_MAX_LEN || new_name.contains('/') {
            return Err(AdfError::InvalidName(format!(
                "Invalid directory name: '{}'",
                new_name
            )));
        }
    
        // Find the parent directory block
        let parent_block = self.find_directory_block(path)?;
    
        // Check if a directory or file with the new name already exists
        let existing_files = self.list_directory(parent_block)?;
        for file in existing_files {
            if file.name == new_name {
                return Err(AdfError::AlreadyExists(format!(
                    "Entry '{}' already exists",
                    new_name
                )));
            }
        }
    
        // Find the directory block to be renamed
        let dir_block = self.find_file_header_block(parent_block, old_name)?;
        
        // Make sure it is actually a directory
        if !self.is_directory(dir_block) {
            return Err(AdfError::InvalidPath(format!(
                "'{}' is not a directory",
                old_name
            )));
        }
    
        // Update the entry in the directory
        self.update_entry_in_directory(parent_block, old_name, new_name)
    }

    pub fn rename_file(
        &mut self,
        path: &str,
        old_name: &str,
        new_name: &str,
    ) -> Result<()> {
        // Check if the new name is valid
        if new_name.is_empty() || new_name.len() > FILE_NAME_MAX_LEN || new_name.contains('/') {
            return Err(AdfError::InvalidName(format!(
                "Invalid file name: '{}'",
                new_name
            )));
        }

        // Find the parent directory block
        let parent_block = self.find_directory_block(path)?;

        // Check if a file with the new name already exists
        let existing_files = self.list_directory(parent_block)?;
        for file in existing_files {
            if file.name == new_name {
                return Err(AdfError::AlreadyExists(format!(
                    "File '{}' already exists",
                    new_name
                )));
            }
        }

        // Update the entry in the directory
        self.update_entry_in_directory(parent_block, old_name, new_name)
    }

    fn is_directory(&self, block: usize) -> bool {
        let block_data = self.read_sector(block);
        block_data[BLOCK_TYPE_OFFSET] == BLOCK_TYPE_DIRECTORY
    }

    fn initialize_bitmap(&mut self) -> Result<()> {
        let mut bitmap_block = [0u8; SECTOR_SIZE];
        bitmap_block[0] = BLOCK_TYPE_FILE;
        bitmap_block[BITMAP_FLAG_OFFSET] = 0xFF;
        bitmap_block[BITMAP_VALID_OFFSET] = 0xFF;

        let checksum = self.calculate_checksum(&bitmap_block[BITMAP_CHECKSUM_OFFSET..]);
        bitmap_block[BITMAP_CHECKSUM_LOCATION..BITMAP_CHECKSUM_LOCATION + 4]
            .copy_from_slice(&checksum.to_be_bytes());

        self.write_sector(BITMAP_BLOCK, &bitmap_block)?;
        self.set_block_allocated(BITMAP_BLOCK, true)?;
        self.update_bitmap_blocks()?;
        Ok(())
    }

    pub fn update_bitmap_blocks(&mut self) -> Result<()> {
        // Calculate the number of bitmap blocks needed
        let bitmap_blocks = (self.bitmap.len() + (SECTOR_SIZE * 8) - 1) / (SECTOR_SIZE * 8);

        for i in 0..bitmap_blocks {
            let bitmap_block = BITMAP_BLOCK + i;
            let mut bitmap_data = [0u8; SECTOR_SIZE];

            // Set the bitmap data
            let start_block = i * (SECTOR_SIZE * 8);
            let end_block = std::cmp::min(start_block + (SECTOR_SIZE * 8), self.bitmap.len());

            for j in start_block..end_block {
                let byte_index = (j - start_block) / 8;
                let bit_index = (j - start_block) % 8;
                if self.bitmap[j] {
                    bitmap_data[byte_index] |= 1 << (7 - bit_index);
                }
            }

            // Write the bitmap block
            self.write_sector(bitmap_block, &bitmap_data)?;
        }

        Ok(())
    }

    fn find_directory_block(&self, path: &str) -> Result<usize> {
        let mut current_block = ROOT_BLOCK;
        for component in path.split('/').filter(|&c| !c.is_empty()) {
            current_block = self.find_file_header_block(current_block, component)?;
            if !self.is_directory(current_block) {
                return Err(AdfError::InvalidPath(format!(
                    "'{}' is not a directory",
                    component
                )));
            }
        }
        Ok(current_block)
    }

    fn add_entry_to_directory(
        &mut self,
        dir_block: usize,
        entry_block: u32,
        name: &str,
    ) -> Result<()> {
        let mut dir_data = self.read_sector(dir_block).to_vec();

        for i in (DIR_ENTRIES_START..=DIR_ENTRIES_END).rev() {
            if u32::from_be_bytes([
                dir_data[i * 4],
                dir_data[i * 4 + 1],
                dir_data[i * 4 + 2],
                dir_data[i * 4 + 3],
            ]) == 0
            {
                dir_data[i * 4..i * 4 + 4].copy_from_slice(&entry_block.to_be_bytes());
                self.write_sector(dir_block, &dir_data)?;
                return Ok(());
            }
        }

        Err(AdfError::DirectoryFull(format!(
            "Directory at block {} is full",
            dir_block
        )))
    }

    fn remove_entry_from_directory(
        &mut self,
        dir_block: usize,
        name: &str,
    ) -> Result<()> {
        let mut dir_data = self.read_sector(dir_block).to_vec();

        for i in (DIR_ENTRIES_START..=DIR_ENTRIES_END).rev() {
            let entry_block = u32::from_be_bytes([
                dir_data[i * 4],
                dir_data[i * 4 + 1],
                dir_data[i * 4 + 2],
                dir_data[i * 4 + 3],
            ]);
            if entry_block != 0 {
                let entry_data = self.read_sector(entry_block as usize);
                let entry_name_len = entry_data[FILE_NAME_LEN_OFFSET] as usize;
                let entry_name = String::from_utf8_lossy(
                    &entry_data[FILE_NAME_OFFSET..FILE_NAME_OFFSET + entry_name_len],
                )
                .to_string();
                if entry_name == name {
                    dir_data[i * 4..i * 4 + 4].copy_from_slice(&0u32.to_be_bytes());
                    self.write_sector(dir_block, &dir_data)?;
                    return Ok(());
                }
            }
        }

        Err(AdfError::NotFound(format!(
            "Entry '{}' not found in directory",
            name
        )))
    }

    fn update_entry_in_directory(
        &mut self,
        dir_block: usize,
        old_name: &str,
        new_name: &str,
    ) -> Result<()> {
        let dir_data = self.read_sector(dir_block);

        for i in (DIR_ENTRIES_START..=DIR_ENTRIES_END).rev() {
            let entry_block = u32::from_be_bytes([
                dir_data[i * 4],
                dir_data[i * 4 + 1],
                dir_data[i * 4 + 2],
                dir_data[i * 4 + 3],
            ]);
            if entry_block != 0 {
                let mut entry_data = self.read_sector(entry_block as usize).to_vec();
                let entry_name_len = entry_data[FILE_NAME_LEN_OFFSET] as usize;
                let entry_name = String::from_utf8_lossy(
                    &entry_data[FILE_NAME_OFFSET..FILE_NAME_OFFSET + entry_name_len],
                )
                .to_string();
                if entry_name == old_name {
                    let new_name_bytes = new_name.as_bytes();
                    let new_name_len = std::cmp::min(new_name_bytes.len(), MAX_NAME_LENGTH);
                    entry_data[FILE_NAME_LEN_OFFSET] = new_name_len as u8;
                    entry_data[FILE_NAME_OFFSET..FILE_NAME_OFFSET + new_name_len]
                        .copy_from_slice(&new_name_bytes[..new_name_len]);
                    self.write_sector(entry_block as usize, &entry_data)?;
                    return Ok(());
                }
            }
        }

        Err(AdfError::NotFound(format!(
            "Entry '{}' not found in directory",
            old_name
        )))
    }
}

fn split_path(path: &str) -> (&str, &str) {
    match path.rfind('/') {
        Some(index) => (&path[..index], &path[index + 1..]),
        None => ("", path),
    }
}
