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
    pub fn extract_file(&self, file_name: &str) -> io::Result<Vec<u8>> {
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
                println!("File: {} (Size: {} bytes)", file_name, file_info.size);
                println!("Header block: sector {}", file_header_block);
                
                let contents = self.read_file_contents(file_header_block)?;
                
                // Basic file type detection
                if contents.iter().all(|&b| b.is_ascii_graphic() || b.is_ascii_whitespace()) {
                    println!("Content (ASCII text):\n{}", String::from_utf8_lossy(&contents));
                } else {
                    println!("Content: Binary data ({} bytes)", contents.len());
                }
                
                return Ok(contents);
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
        self.list_directory(ROOT_BLOCK)
    }

    pub fn list_directory(&self, block: usize) -> Result<Vec<FileInfo>> {
        let block_data = self.read_sector(block);
        let mut files = Vec::new();

        for i in (24..=51).rev() {
            let sector = u32::from_be_bytes([
                block_data[i * 4],
                block_data[i * 4 + 1],
                block_data[i * 4 + 2],
                block_data[i * 4 + 3],
            ]);
            if sector != 0 {
                let file_info = self.read_file_header(sector as usize)?;
                files.push(file_info);
            }
        }

        Ok(files)
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

        let secs = days * 86400 + mins * 60 + ticks / 50;
        let creation_date = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs as u64);

        Ok(FileInfo {
            name,
            size,
            is_dir,
            protection,
            creation_date,
        })
    }

    pub fn read_file_contents(&self, block: usize) -> io::Result<Vec<u8>> {
        let block_data = self.read_sector(block);

        match block_data[0] {
            2 => {
                let file_size = u32::from_be_bytes([block_data[4], block_data[5], block_data[6], block_data[7]]) as usize;
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
                    current_block = u32::from_be_bytes([data_block[0], data_block[1], data_block[2], data_block[3]]) as usize;
                }

                if contents.len() != file_size {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        format!("File size mismatch. Expected: {}, Read: {}", file_size, contents.len()),
                    ));
                }

                Ok(contents)
            },
            0 => {
                // Handle type 0 blocks (possibly direct data blocks)
                let file_size = u32::from_be_bytes([block_data[4], block_data[5], block_data[6], block_data[7]]) as usize;
                let mut contents = Vec::with_capacity(file_size);
                contents.extend_from_slice(&block_data[24..]);

                let mut current_block = u32::from_be_bytes([block_data[16], block_data[17], block_data[18], block_data[19]]) as usize;
                while current_block != 0 && contents.len() < file_size {
                    let data_block = self.read_sector(current_block);
                    let data_size = std::cmp::min(512, file_size - contents.len());
                    contents.extend_from_slice(&data_block[..data_size]);
                    current_block = u32::from_be_bytes([data_block[0], data_block[1], data_block[2], data_block[3]]) as usize;
                }

                Ok(contents)
            },
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unexpected block type: {}", block_data[0]),
            )),
        }
    }

    fn write_boot_block(&mut self, disk_type: DiskType) -> Result<()> {
        let mut boot_block = [0u8; ADF_SECTOR_SIZE * 2];

        boot_block[0] = b'D';
        boot_block[1] = b'O';
        boot_block[2] = b'S';

        boot_block[3] = match disk_type {
            DiskType::OFS => 0,
            DiskType::FFS => 1,
        };

        self.data[..ADF_SECTOR_SIZE * 2].copy_from_slice(&boot_block);
        Ok(())
    }

    fn write_root_block(&mut self, disk_type: DiskType, disk_name: &str) -> Result<()> {
        let mut root_block = [0u8; ADF_SECTOR_SIZE];

        // Block type (2 = T_HEADER)
        root_block[0] = 2;

        // Disk type
        root_block[ADF_SECTOR_SIZE - 4] = match disk_type {
            DiskType::OFS => 0,
            DiskType::FFS => 1,
        };

        // Hash table size (72 entries for Amiga floppy disks)
        root_block[12] = 0;
        root_block[13] = 72;

        // Bitmap flag and bitmap blocks
        if matches!(disk_type, DiskType::FFS) {
            root_block[ADF_SECTOR_SIZE - 200] = 0xFF; // Bitmap flag
                                                      // Set bitmap pointers (blocks 881, 882, ...)
            for i in 0..25 {
                let block_num = ROOT_BLOCK as u32 + 1 + i as u32;
                root_block[ADF_SECTOR_SIZE - 196 + i * 4..ADF_SECTOR_SIZE - 192 + i * 4]
                    .copy_from_slice(&block_num.to_be_bytes());
            }
        }

        // Disk name
        let name_bytes = disk_name.as_bytes();
        root_block[ADF_SECTOR_SIZE - 80] = name_bytes.len() as u8;
        root_block[ADF_SECTOR_SIZE - 79..ADF_SECTOR_SIZE - 79 + name_bytes.len()]
            .copy_from_slice(name_bytes);

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let days = (now.as_secs() / 86400) as u32;
        let mins = ((now.as_secs() % 86400) / 60) as u32;
        let ticks = ((now.as_secs() % 60) * 50) as u32;

        root_block[ADF_SECTOR_SIZE - 92..ADF_SECTOR_SIZE - 88].copy_from_slice(&days.to_be_bytes());
        root_block[ADF_SECTOR_SIZE - 88..ADF_SECTOR_SIZE - 84].copy_from_slice(&mins.to_be_bytes());
        root_block[ADF_SECTOR_SIZE - 84..ADF_SECTOR_SIZE - 80]
            .copy_from_slice(&ticks.to_be_bytes());

        self.write_sector(ROOT_BLOCK, &root_block)?;
        Ok(())
    }

    fn write_bitmap_blocks(&mut self) -> Result<()> {
        let mut bitmap_block = [0xFFu8; ADF_SECTOR_SIZE];

        bitmap_block[0] = 0xF8; // 11111000
        bitmap_block[1] = 0xFF; // 11111111
        bitmap_block[2] = 0xFF; // 11111111

        self.write_sector(ROOT_BLOCK + 1, &bitmap_block)?;
        self.write_sector(ROOT_BLOCK + 2, &[0xFFu8; ADF_SECTOR_SIZE])?;

        Ok(())
    }
    pub fn information(&self) -> Result<String> {
        let mut info = String::new();

        let boot_block = self.read_sector(0);
        let dos_type = match &boot_block[0..3] {
            b"DOS" => {
                let fs_flag = boot_block[3];
                match fs_flag {
                    0 => "OFS (Old File System)",
                    1 => "FFS (Fast File System)",
                    _ => "Unknown",
                }
            }
            _ => "Not a DOS disk",
        };

        info.push_str(&format!("Filesystem: {}\n", dos_type));

        let root_block = self.read_sector(ROOT_BLOCK);

        let name_len = root_block[ADF_SECTOR_SIZE - 80] as usize;
        let name = String::from_utf8_lossy(
            &root_block[ADF_SECTOR_SIZE - 79..ADF_SECTOR_SIZE - 79 + name_len],
        );
        info.push_str(&format!("Disk Name: {}\n", name));

        // Creation date
        let days = u32::from_be_bytes([
            root_block[ADF_SECTOR_SIZE - 92],
            root_block[ADF_SECTOR_SIZE - 91],
            root_block[ADF_SECTOR_SIZE - 90],
            root_block[ADF_SECTOR_SIZE - 89],
        ]);
        let mins = u32::from_be_bytes([
            root_block[ADF_SECTOR_SIZE - 88],
            root_block[ADF_SECTOR_SIZE - 87],
            root_block[ADF_SECTOR_SIZE - 86],
            root_block[ADF_SECTOR_SIZE - 85],
        ]);
        let ticks = u32::from_be_bytes([
            root_block[ADF_SECTOR_SIZE - 84],
            root_block[ADF_SECTOR_SIZE - 83],
            root_block[ADF_SECTOR_SIZE - 82],
            root_block[ADF_SECTOR_SIZE - 81],
        ]);

        let creation_date = SystemTime::UNIX_EPOCH
            + std::time::Duration::from_secs(
                (days as u64 * 86400) + (mins as u64 * 60) + (ticks as u64 / 50),
            );
        info.push_str(&format!(
            "Creation Date: {}\n",
            creation_date.duration_since(UNIX_EPOCH).unwrap().as_secs()
        ));

        // Disk geometry
        info.push_str(&format!("Disk Size: {} bytes\n", self.data.len()));
        info.push_str(&format!("Heads: 2\n"));
        info.push_str(&format!("Tracks: {}\n", ADF_NUM_TRACKS / 2));
        info.push_str(&format!(
            "Sectors per Track: {}\n",
            ADF_TRACK_SIZE / ADF_SECTOR_SIZE
        ));
        info.push_str(&format!("Bytes per Sector: {}\n", ADF_SECTOR_SIZE));

        let hash_size = u32::from_be_bytes([0, 0, root_block[12], root_block[13]]);
        info.push_str(&format!("Hash Table Size: {}\n", hash_size));

        info.push_str("Reserved Blocks:\n");
        info.push_str(&format!("  First: {}\n", 0)); // Boot block starts at sector 0
        info.push_str(&format!("  Last: {}\n", ROOT_BLOCK)); // Root block

        if dos_type.starts_with("FFS") {
            let bitmap_flag = root_block[ADF_SECTOR_SIZE - 200];
            info.push_str(&format!("Bitmap Flag: 0x{:02X}\n", bitmap_flag));

            info.push_str("Bitmap Blocks: ");
            for i in 0..25 {
                let block_num = u32::from_be_bytes([
                    root_block[ADF_SECTOR_SIZE - 196 + i * 4],
                    root_block[ADF_SECTOR_SIZE - 195 + i * 4],
                    root_block[ADF_SECTOR_SIZE - 194 + i * 4],
                    root_block[ADF_SECTOR_SIZE - 193 + i * 4],
                ]);
                if block_num != 0 {
                    info.push_str(&format!("{} ", block_num));
                }
            }
            info.push('\n');
        }

        Ok(info)
    }

    pub fn list(&self) -> Result<String> {
        let mut output = String::new();

        let files = self.list_root_directory()?;

        for file in files {
            output.push_str(&format!("{} ({} bytes)\n", file.name, file.size));
        }

        Ok(output)
    }
}
