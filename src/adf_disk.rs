/*
Copyright 2022 Volker Schwaberow <volker@schwaberow.de>
Permission is hereby granted, free of charge, to any person obtaining a
copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including without
limitation the rights to use, copy, modify, merge, publish, distribute,
sublicense, and/or sell copies of the Software, and to permit persons to whom the
Software is furnished to do so, subject to the following conditions:
The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.
THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR
OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
Author(s): Volker Schwaberow
*/

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
}

