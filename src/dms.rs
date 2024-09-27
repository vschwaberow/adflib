// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2023
// - Volker Schwaberow <volker@schwaberow.de>

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;

const DMS_HEADER_SIZE: usize = 56;
const DMS_TRACK_HEADER_SIZE: usize = 20;
const QBITMASK: u16 = 255;

#[derive(Debug, Clone)]
pub struct DMSHeader {
    pub signature: [u8; 4],
    pub archive_header_len: u32,
    pub version: u16,
    pub disk_type: u8,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub track_count: u16,
    pub flags: u16,
    pub low_track: u16,
    pub high_track: u16,
    pub creation_date: u32,
    pub creation_time: u32,
    pub name: [u8; 20],
    pub packing_mode: DMSPackingMode,
}

#[derive(Debug, Clone)]
pub enum DMSPackingMode {
    None,
    Simple,
    Quick,
    Medium,
    Deep,
    Heavy1,
    Heavy2,
}

impl From<u8> for DMSPackingMode {
    fn from(value: u8) -> Self {
        match value {
            0 => DMSPackingMode::None,
            1 => DMSPackingMode::Simple,
            2 => DMSPackingMode::Quick,
            3 => DMSPackingMode::Medium,
            4 => DMSPackingMode::Deep,
            5 => DMSPackingMode::Heavy1,
            6 => DMSPackingMode::Heavy2,
            _ => DMSPackingMode::None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DMSInfo {
    pub signature: String,
    pub version: u16,
    pub disk_type: u8,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub track_count: u16,
    pub low_track: u16,
    pub high_track: u16,
    pub creation_date: u32,
    pub creation_time: u32,
    pub name: String,
    pub packing_mode: DMSPackingMode,
}

#[derive(Debug)]
pub struct TrackInfo {
    pub number: u16,
    pub packing_mode: DMSPackingMode,
    pub data_size: u32,
}

#[derive(Debug, Clone)]
pub struct DMSTrackHeader {
    pub id: u16,
    pub track_number: u16,
    pub flags: u8,
    pub packing_mode: u8,
    pub data_size: u32,
}

#[derive(Debug, Clone)]
pub struct DMSReader<R: Read + Seek> {
    reader: R,
    header: DMSHeader,
    bit_buffer: u32,
    bit_count: u8,
    quick_text_loc: u16,
    text: [u8; 256],
}

impl<R: Read + Seek> DMSReader<R> {
    pub fn new(mut reader: R) -> io::Result<Self> {
        let header = Self::read_header(&mut reader)?;
        Ok(Self {
            reader,
            header,
            bit_buffer: 0,
            bit_count: 0,
            quick_text_loc: 0,
            text: [0; 256],
        })
    }

    pub fn load_disk_info(&mut self) -> io::Result<DMSInfo> {
        let dms_info = self.info();
        let mut tracks = Vec::new();
        self.reader.seek(SeekFrom::Start(DMS_HEADER_SIZE as u64))?;

        for _ in 0..dms_info.track_count {
            let track_header = self.read_track_header()?;
            tracks.push(TrackInfo {
                number: track_header.track_number,
                packing_mode: DMSPackingMode::from(track_header.packing_mode),
                data_size: track_header.data_size,
            });

            self.reader
                .seek(SeekFrom::Current(track_header.data_size as i64))?;
        }

        Ok(dms_info)
    }
    pub fn info(&self) -> DMSInfo {
        DMSInfo {
            signature: String::from_utf8_lossy(&self.header.signature).to_string(),
            version: self.header.version,
            disk_type: self.header.disk_type,
            compressed_size: self.header.compressed_size,
            packing_mode: DMSPackingMode::from(self.header.packing_mode.clone()),
            uncompressed_size: self.header.uncompressed_size,
            track_count: self.header.track_count,
            low_track: self.header.low_track,
            high_track: self.header.high_track,
            creation_date: self.header.creation_date,
            creation_time: self.header.creation_time,
            name: String::from_utf8_lossy(&self.header.name)
                .trim_end_matches('\0')
                .to_string(),
        }
    }

    fn read_header(reader: &mut R) -> io::Result<DMSHeader> {
        let mut signature = [0u8; 4];
        reader.read_exact(&mut signature)?;

        if &signature != b"DMS!" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid DMS signature",
            ));
        }

        Ok(DMSHeader {
            signature,
            archive_header_len: reader.read_u32::<BigEndian>()?,
            version: reader.read_u16::<BigEndian>()?,
            disk_type: reader.read_u8()?,
            packing_mode: DMSPackingMode::try_from(reader.read_u8()?)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            compressed_size: reader.read_u32::<BigEndian>()?,
            uncompressed_size: reader.read_u32::<BigEndian>()?,
            track_count: reader.read_u16::<BigEndian>()?,
            flags: reader.read_u16::<BigEndian>()?,
            low_track: reader.read_u16::<BigEndian>()?,
            high_track: reader.read_u16::<BigEndian>()?,
            creation_date: reader.read_u32::<BigEndian>()?,
            creation_time: reader.read_u32::<BigEndian>()?,
            name: {
                let mut name = [0u8; 20];
                reader.read_exact(&mut name)?;
                name
            },
        })
    }

    pub fn read_track(&mut self) -> io::Result<Vec<u8>> {
        let track_header = self.read_track_header()?;
        let mut compressed_data = vec![0u8; track_header.data_size as usize];
        self.reader.read_exact(&mut compressed_data)?;

        let packing_mode = DMSPackingMode::from(track_header.packing_mode);
        match packing_mode {
            DMSPackingMode::None => {
                return Ok(compressed_data);
            }
            DMSPackingMode::Simple => {
                return self.unpack_rle(&compressed_data);
            }
            DMSPackingMode::Quick => {
                return self.unpack_quick(&compressed_data);
            }
            DMSPackingMode::Medium => (),
            DMSPackingMode::Deep => (),
            DMSPackingMode::Heavy1 => (),
            DMSPackingMode::Heavy2 => (),
        }

        Ok(compressed_data)
    }

    fn unpack_rle(&self, input: &[u8]) -> io::Result<Vec<u8>> {
        let mut output = Vec::new();
        let mut i = 0;

        while i < input.len() {
            let a = input[i];
            i += 1;

            if a != 0x90 {
                output.push(a);
            } else {
                if i >= input.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Unexpected end of input",
                    ));
                }
                let b = input[i];
                i += 1;

                if b == 0 {
                    output.push(a);
                } else {
                    if i >= input.len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Unexpected end of input",
                        ));
                    }
                    let rep_char = input[i];
                    i += 1;

                    let rep_count = if b == 0xff {
                        if i + 1 >= input.len() {
                            return Err(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "Unexpected end of input",
                            ));
                        }
                        let n = u16::from_be_bytes([input[i], input[i + 1]]);
                        i += 2;
                        n as usize
                    } else {
                        b as usize
                    };

                    output.extend(std::iter::repeat(rep_char).take(rep_count));
                }
            }
        }

        Ok(output)
    }

    fn unpack_quick(&mut self, input: &[u8]) -> io::Result<Vec<u8>> {
        let mut output = Vec::new();
        self.init_bit_buf(input);

        while output.len() < 11360 {
            // Typical Amiga floppy track size
            if self.get_bits(1) != 0 {
                self.drop_bits(1);
                let byte = self.get_bits(8) as u8;
                self.drop_bits(8);
                self.text[self.quick_text_loc as usize & QBITMASK as usize] = byte;
                self.quick_text_loc = self.quick_text_loc.wrapping_add(1);
                output.push(byte);
            } else {
                self.drop_bits(1);
                let j = self.get_bits(2) as usize + 2;
                self.drop_bits(2);
                let i = self
                    .quick_text_loc
                    .wrapping_sub(self.get_bits(8) as u16)
                    .wrapping_sub(1);
                self.drop_bits(8);
                for _ in 0..j {
                    let byte = self.text[i as usize & QBITMASK as usize];
                    self.text[self.quick_text_loc as usize & QBITMASK as usize] = byte;
                    self.quick_text_loc = self.quick_text_loc.wrapping_add(1);
                    output.push(byte);
                }
            }
        }

        self.quick_text_loc = self.quick_text_loc.wrapping_add(5) & QBITMASK;
        Ok(output)
    }

    fn init_bit_buf(&mut self, input: &[u8]) {
        self.bit_buffer = u32::from_be_bytes([input[0], input[1], input[2], input[3]]);
        self.bit_count = 32;
    }

    fn get_bits(&mut self, n: u8) -> u32 {
        (self.bit_buffer >> (32 - n)) & ((1 << n) - 1)
    }

    fn drop_bits(&mut self, n: u8) {
        self.bit_buffer <<= n;
        self.bit_count -= n;
        if self.bit_count <= 24 {
            let mut next_byte = [0u8; 1];
            if self.reader.read_exact(&mut next_byte).is_ok() {
                self.bit_buffer |= (next_byte[0] as u32) << (24 - self.bit_count);
                self.bit_count += 8;
            }
        }
    }

    fn read_track_header(&mut self) -> io::Result<DMSTrackHeader> {
        Ok(DMSTrackHeader {
            id: self.reader.read_u16::<BigEndian>()?,
            track_number: self.reader.read_u16::<BigEndian>()?,
            flags: self.reader.read_u8()?,
            packing_mode: self.reader.read_u8()?,
            data_size: self.reader.read_u32::<BigEndian>()?,
        })
    }

    pub fn read_u16(&mut self) -> io::Result<u16> {
        let mut buf = [0u8; 2];
        self.reader.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    pub fn read_u32(&mut self) -> io::Result<u32> {
        let mut buf = [0u8; 4];
        self.reader.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }

    pub fn read_u8(&mut self) -> io::Result<u8> {
        let mut buf = [0u8; 1];
        self.reader.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    pub fn read_sector(&mut self, sector: usize) -> io::Result<Vec<u8>> {
        let track = sector / 16;
        let sector_in_track = sector % 16;

        for _ in 0..track {
            self.read_track()?;
        }

        let track_data = self.read_track()?;
        let sector_data = track_data[sector_in_track * 256..(sector_in_track + 1) * 256].to_vec();
        Ok(sector_data)
    }
}

pub fn dms_to_adf<R: Read + Seek, W: Write>(reader: R, writer: &mut W) -> io::Result<()> {
    let mut dms_reader = DMSReader::new(reader)?;

    for _ in 0..dms_reader.header.track_count {
        let track_data = dms_reader.read_track()?;
        writer.write_all(&track_data)?;
    }

    Ok(())
}

pub fn convert_dms_to_adf(dms_path: &str, adf_path: &str) -> io::Result<()> {
    let dms_file = File::open(dms_path)?;
    let mut adf_file = File::create(adf_path)?;

    dms_to_adf(dms_file, &mut adf_file)
}
