// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2023
// - Volker Schwaberow <volker@schwaberow.de>

use std::fs::File;
use std::io::{Read, Result, Write};

const ADF_SECTOR_SIZE: usize = 512;
const ADF_TRACK_SIZE: usize = 11 * ADF_SECTOR_SIZE;
const ADF_NUM_TRACKS: usize = 80 * 2;

#[derive(Debug, Clone)]
pub struct ADF {
    data: Vec<u8>
}

impl ADF {
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

    pub fn read_sector(&self, track: usize, sector: usize) -> &[u8] {
        let offset = track * ADF_TRACK_SIZE + sector * ADF_SECTOR_SIZE;
        &self.data[offset..offset + ADF_SECTOR_SIZE]
    }

    pub fn write_sector(&mut self, track: usize, sector: usize, data: &[u8]) -> Result<()> {
        if data.len() != ADF_SECTOR_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid sector data size",
            ));
        }
        let offset = track * ADF_TRACK_SIZE + sector * ADF_SECTOR_SIZE;
        self.data[offset..offset + ADF_SECTOR_SIZE].copy_from_slice(data);
        Ok(())
    }

    pub fn read_track(&self, track: usize) -> &[u8] {
        let offset = track * ADF_TRACK_SIZE;
        &self.data[offset..offset + ADF_TRACK_SIZE]
    }

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
}
