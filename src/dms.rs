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
const QUICK_UNPACK_SIZE: usize = 11360;

#[derive(Debug, Clone)]
pub struct DMSHeader {
    pub signature: [u8; 4],
    pub header_type: [u8; 4],
    pub info_bits: u32,
    pub date: u32,
    pub low_track: u16,
    pub high_track: u16,
    pub packed_size: u32,
    pub unpacked_size: u32,
    pub os_version: u16,
    pub os_revision: u16,
    pub machine_cpu: u16,
    pub cpu_copro: u16,
    pub machine_type: u16,
    pub unused: u16,
    pub cpu_mhz: u16,
    pub time_create: u32,
    pub version_creator: u16,
    pub version_needed: u16,
    pub diskette_type: u16,
    pub compression_mode: u16,
    pub info_header_crc: u16,
}

#[derive(Debug, Clone, Copy)]
pub enum DMSPackingMode {
    None,
    Simple,
    Quick,
    Medium,
    Deep,
    Heavy1,
    Heavy2,
    Heavy3,
    Heavy4,
    Heavy5,
    Unsupported,
}

impl From<u16> for DMSPackingMode {
    fn from(value: u16) -> Self {
        match value {
            0 => DMSPackingMode::None,
            1 => DMSPackingMode::Simple,
            2 => DMSPackingMode::Quick,
            3 => DMSPackingMode::Medium,
            4 => DMSPackingMode::Deep,
            5 => DMSPackingMode::Heavy1,
            6 => DMSPackingMode::Heavy2,
            7 => DMSPackingMode::Heavy3,
            8 => DMSPackingMode::Heavy4,
            9 => DMSPackingMode::Heavy5,
            _ => DMSPackingMode::Unsupported,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DMSInfo {
    pub signature: String,
    pub header_type: String,
    pub info_bits: u32,
    pub date: u32,
    pub low_track: u16,
    pub high_track: u16,
    pub packed_size: u32,
    pub unpacked_size: u32,
    pub compression_mode: DMSPackingMode,
}

#[derive(Debug, Clone)]
pub struct DMSTrackHeader {
    pub header_id: [u8; 2],
    pub track_number: u16,
    pub unused1: u16,
    pub pack_length: u16,
    pub unused2: u16,
    pub unpack_length: u16,
    pub c_flag: u8,
    pub packing_mode: DMSPackingMode,
    pub u_sum: u16,
    pub d_crc: u16,
    pub h_crc: u16,
}

pub struct DMSReader<R: Read + Seek> {
    reader: R,
    header: DMSHeader,
    bit_buffer: u32,
    bit_count: u8,
    quick_text_loc: u16,
    text: [u8; 256],
}

impl std::fmt::Display for DMSPackingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DMSPackingMode::None => write!(f, "None"),
            DMSPackingMode::Simple => write!(f, "Simple"),
            DMSPackingMode::Quick => write!(f, "Quick"),
            DMSPackingMode::Medium => write!(f, "Medium"),
            DMSPackingMode::Deep => write!(f, "Deep"),
            DMSPackingMode::Heavy1 => write!(f, "Heavy1"),
            DMSPackingMode::Heavy2 => write!(f, "Heavy2"),
            DMSPackingMode::Heavy3 => write!(f, "Heavy3"),
            DMSPackingMode::Heavy4 => write!(f, "Heavy4"),
            DMSPackingMode::Heavy5 => write!(f, "Heavy5"),
            DMSPackingMode::Unsupported => write!(f, "Unsupported"),
        }
    }
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

    fn read_header(reader: &mut R) -> io::Result<DMSHeader> {
        let mut signature = [0u8; 4];
        reader.read_exact(&mut signature)?;
        if &signature != b"DMS!" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid DMS signature",
            ));
        }
        let mut header_type = [0u8; 4];
        reader.read_exact(&mut header_type)?;
        Ok(DMSHeader {
            signature,
            header_type,
            info_bits: reader.read_u32::<BigEndian>()?,
            date: reader.read_u32::<BigEndian>()?,
            low_track: reader.read_u16::<BigEndian>()?,
            high_track: reader.read_u16::<BigEndian>()?,
            packed_size: reader.read_u32::<BigEndian>()?,
            unpacked_size: reader.read_u32::<BigEndian>()?,
            os_version: reader.read_u16::<BigEndian>()?,
            os_revision: reader.read_u16::<BigEndian>()?,
            machine_cpu: reader.read_u16::<BigEndian>()?,
            cpu_copro: reader.read_u16::<BigEndian>()?,
            machine_type: reader.read_u16::<BigEndian>()?,
            unused: reader.read_u16::<BigEndian>()?,
            cpu_mhz: reader.read_u16::<BigEndian>()?,
            time_create: reader.read_u32::<BigEndian>()?,
            version_creator: reader.read_u16::<BigEndian>()?,
            version_needed: reader.read_u16::<BigEndian>()?,
            diskette_type: reader.read_u16::<BigEndian>()?,
            compression_mode: reader.read_u16::<BigEndian>()?,
            info_header_crc: reader.read_u16::<BigEndian>()?,
        })
    }

    pub fn info(&self) -> DMSInfo {
        DMSInfo {
            signature: String::from_utf8_lossy(&self.header.signature).to_string(),
            header_type: String::from_utf8_lossy(&self.header.header_type).to_string(),
            info_bits: self.header.info_bits,
            date: self.header.date,
            low_track: self.header.low_track,
            high_track: self.header.high_track,
            packed_size: self.header.packed_size,
            unpacked_size: self.header.unpacked_size,
            compression_mode: DMSPackingMode::from(self.header.compression_mode),
        }
    }

    pub fn read_track(&mut self) -> io::Result<Vec<u8>> {
        let track_header = self.read_track_header()?;
        let mut compressed_data = vec![0u8; track_header.pack_length as usize];
        self.reader.read_exact(&mut compressed_data)?;
        match track_header.packing_mode {
            DMSPackingMode::None => Ok(compressed_data),
            DMSPackingMode::Simple => self.unpack_rle(&compressed_data),
            DMSPackingMode::Quick => self.unpack_quick(&compressed_data),
            _ => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Unsupported packing mode",
            )),
        }
    }

    fn read_track_header(&mut self) -> io::Result<DMSTrackHeader> {
        let mut header_id = [0u8; 2];
        self.reader.read_exact(&mut header_id)?;
        if &header_id != b"TR" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid track header ID",
            ));
        }
        Ok(DMSTrackHeader {
            header_id,
            track_number: self.reader.read_u16::<BigEndian>()?,
            unused1: self.reader.read_u16::<BigEndian>()?,
            pack_length: self.reader.read_u16::<BigEndian>()?,
            unused2: self.reader.read_u16::<BigEndian>()?,
            unpack_length: self.reader.read_u16::<BigEndian>()?,
            c_flag: self.reader.read_u8()?,
            packing_mode: DMSPackingMode::from(self.reader.read_u8()? as u16),
            u_sum: self.reader.read_u16::<BigEndian>()?,
            d_crc: self.reader.read_u16::<BigEndian>()?,
            h_crc: self.reader.read_u16::<BigEndian>()?,
        })
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

        while output.len() < QUICK_UNPACK_SIZE {
            if self.get_bits(1) != 0 {
                self.drop_bits(1);
                let byte = self.get_bits(8) as u8;
                self.drop_bits(8);
                self.text[self.quick_text_loc as usize & 255] = byte;
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

                output.reserve(j);
                for _ in 0..j {
                    let byte = self.text[i as usize & 255];
                    self.text[self.quick_text_loc as usize & 255] = byte;
                    self.quick_text_loc = self.quick_text_loc.wrapping_add(1);
                    output.push(byte);
                }
            }
        }

        self.quick_text_loc = (self.quick_text_loc.wrapping_add(5)) & 255;
        Ok(output)
    }

    fn init_bit_buf(&mut self, input: &[u8]) {
        self.bit_buffer = u32::from_be_bytes([input[0], input[1], input[2], input[3]]);
        self.bit_count = 32;
    }

    fn get_bits(&self, n: u8) -> u32 {
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

    pub fn read_sector(&mut self, sector: usize) -> io::Result<Vec<u8>> {
        let track = sector / 16;
        let sector_in_track = sector % 16;
        for _ in 0..track {
            self.read_track()?;
        }
        let track_data = self.read_track()?;
        Ok(track_data[sector_in_track * 256..(sector_in_track + 1) * 256].to_vec())
    }
}

pub fn dms_to_adf<R: Read + Seek, W: Write>(reader: R, writer: &mut W) -> io::Result<()> {
    let mut dms_reader = DMSReader::new(reader)?;
    for _ in 0..dms_reader.header.high_track - dms_reader.header.low_track + 1 {
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
