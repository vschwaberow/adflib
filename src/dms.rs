// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2023
// - Volker Schwaberow <volker@schwaberow.de>

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::env;
use std::fs::File;
use std::io::{self, Read, Seek, Write};
use std::process;

const DMS_HEADER_SIZE_BYTES: usize = 56;
const DMS_TRACK_HEADER_SIZE_BYTES: usize = 20;
const QUICK_TEXT_MASK: u16 = 255;
const QUICK_UNPACK_SIZE_BYTES: usize = 11360;
const SECTORS_PER_TRACK: usize = 16;
const BYTES_PER_SECTOR: usize = 256;

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
    pub info_bits: InfoBits,
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

#[derive(Debug, Clone, Copy)]
pub struct InfoBits(u32);

impl InfoBits {
    pub const NOZERO: u32 = 0x00000001;
    pub const ENCRYPT: u32 = 0b00000010;
    pub const APPENDS: u32 = 0b00000100;
    pub const BANNER: u32 = 0b00001000;
    pub const HIGHDENSITY: u32 = 0b00010000;
    pub const PC: u32 = 0b00100000;
    pub const DMS_DEVICE_FIX: u32 = 0b01000000;
    pub const FILE_ID_DIZ: u32 = 0b100000000;

    pub fn new(bits: u32) -> Self {
        InfoBits(bits)
    }

    pub fn contains(&self, flag: u32) -> bool {
        self.0 & flag != 0
    }
}

impl std::fmt::Display for InfoBits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "InfoBits: {:#010x}", self.0)?;
        writeln!(f, "Flags:")?;
        if self.0 == 0 {
            writeln!(f, "  - No flags set")?;
        } else {
            let mut flag_set = false;
            if self.contains(Self::NOZERO) {
                writeln!(f, "  - NOZERO: No zero compression")?;
                flag_set = true;
            }
            if self.contains(Self::ENCRYPT) {
                writeln!(f, "  - ENCRYPT: File is encrypted")?;
                flag_set = true;
            }
            if self.contains(Self::APPENDS) {
                writeln!(f, "  - APPENDS: File has appended data")?;
                flag_set = true;
            }
            if self.contains(Self::BANNER) {
                writeln!(f, "  - BANNER: File includes a banner")?;
                flag_set = true;
            }
            if self.contains(Self::HIGHDENSITY) {
                writeln!(f, "  - HIGHDENSITY: High-density disk")?;
                flag_set = true;
            }
            if self.contains(Self::PC) {
                writeln!(f, "  - PC: Intended for PC systems")?;
                flag_set = true;
            }
            if self.contains(Self::DMS_DEVICE_FIX) {
                writeln!(f, "  - DMS_DEVICE_FIX: Device-specific fix")?;
                flag_set = true;
            }
            if self.contains(Self::FILE_ID_DIZ) {
                writeln!(f, "  - FILE_ID_DIZ: Includes FILE_ID.DIZ")?;
                flag_set = true;
            }
            if !flag_set {
                writeln!(f, "  - Unknown flags set")?;
            }
        }
        Ok(())
    }
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

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    bit_buffer: u32,
    bit_count: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        let init = if data.len() >= 4 {
            u32::from_be_bytes([data[0], data[1], data[2], data[3]])
        } else {
            0
        };
        Self {
            data,
            pos: 4,
            bit_buffer: init,
            bit_count: 32,
        }
    }

    fn ensure_bits(&mut self, n: u8) {
        while self.bit_count < n && self.pos < self.data.len() {
            self.bit_buffer = (self.bit_buffer << 8) | self.data[self.pos] as u32;
            self.pos += 1;
            self.bit_count += 8;
        }
    }

    fn get_bits(&mut self, n: u8) -> io::Result<u32> {
        self.ensure_bits(n);
        if self.bit_count < n {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Not enough bits in stream",
            ));
        }
        let shift = self.bit_count - n;
        let mask = (1 << n) - 1;
        self.bit_count -= n;
        Ok((self.bit_buffer >> shift) & mask)
    }
}

pub struct DMSReader<R: Read + Seek> {
    reader: R,
    header: DMSHeader,
    quick_text_loc: u8,
    text: [u8; 256],
}

impl<R: Read + Seek> DMSReader<R> {
    pub fn new(mut reader: R) -> io::Result<Self> {
        let header = Self::read_header(&mut reader)?;
        Ok(Self {
            reader,
            header,
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
            info_bits: InfoBits::new(self.header.info_bits),
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
        let mut output = Vec::with_capacity(QUICK_UNPACK_SIZE_BYTES);
        let mut bit_reader = BitReader::new(input);
        while output.len() < QUICK_UNPACK_SIZE_BYTES {
            if bit_reader.get_bits(1)? != 0 {
                let byte = bit_reader.get_bits(8)? as u8;
                self.text[self.quick_text_loc as usize] = byte;
                self.quick_text_loc = self.quick_text_loc.wrapping_add(1);
                output.push(byte);
            } else {
                let j = (bit_reader.get_bits(2)? as usize) + 2;
                let offset = bit_reader.get_bits(8)? as u8;
                let i = self.quick_text_loc.wrapping_sub(offset).wrapping_sub(1);
                for _ in 0..j {
                    let idx = i as usize & 0xff;
                    let byte = self.text[idx];
                    self.text[self.quick_text_loc as usize & 0xff] = byte;
                    self.quick_text_loc = self.quick_text_loc.wrapping_add(1);
                    output.push(byte);
                }
            }
        }
        self.quick_text_loc = self.quick_text_loc.wrapping_add(5) & 0xff;
        Ok(output)
    }

    pub fn read_sector(&mut self, sector: usize) -> io::Result<Vec<u8>> {
        let track = sector / SECTORS_PER_TRACK;
        let sector_in_track = sector % SECTORS_PER_TRACK;
        for _ in 0..track {
            self.read_track()?;
        }
        let track_data = self.read_track()?;
        let start = sector_in_track * BYTES_PER_SECTOR;
        let end = start + BYTES_PER_SECTOR;
        Ok(track_data[start..end].to_vec())
    }
}

pub fn dms_to_adf<R: Read + Seek, W: Write>(reader: R, writer: &mut W) -> io::Result<()> {
    let mut dms_reader = DMSReader::new(reader)?;
    let tracks = dms_reader.header.high_track - dms_reader.header.low_track + 1;
    for _ in 0..tracks {
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
