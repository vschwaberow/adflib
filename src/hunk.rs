// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2023
// - Volker Schwaberow <volker@schwaberow.de>

use std::fmt;
use std::fs::File;
use std::io::{self, BufReader, Error, ErrorKind, Read, Seek, SeekFrom};
use std::path::Path;

const HUNK_HEADER: u32 = 1011;
const HUNK_CODE: u32 = 1001;
const HUNK_DATA: u32 = 1002;
const HUNK_BSS: u32 = 1003;
const HUNK_RELOC32: u32 = 1004;
const HUNK_DEBUG: u32 = 1009;
const HUNK_SYMBOL: u32 = 1008;
const HUNK_END: u32 = 1010;
const DEBUG_LINE: u32 = 0x4c494e45;

const HUNKF_CHIP: u32 = 1 << 30;
const HUNKF_FAST: u32 = 1 << 31;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunkType {
    Code,
    Data,
    Bss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    Any,
    Chip,
    Fast,
}

#[derive(Debug, Clone)]
pub struct RelocInfo32 {
    pub target: usize,
    pub offsets: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub offset: u32,
}

#[derive(Debug, Clone)]
pub struct SourceLine {
    pub line: u32,
    pub offset: u32,
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub name: String,
    pub base_offset: u32,
    pub lines: Vec<SourceLine>,
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub mem_type: MemoryType,
    pub hunk_type: HunkType,
    pub alloc_size: usize,
    pub data_size: usize,
    pub code_data: Option<Vec<u8>>,
    pub reloc_32: Option<Vec<RelocInfo32>>,
    pub symbols: Option<Vec<Symbol>>,
    pub line_debug_info: Option<Vec<SourceFile>>,
}

impl Default for Hunk {
    fn default() -> Self {
        Self {
            mem_type: MemoryType::Any,
            hunk_type: HunkType::Code,
            alloc_size: 0,
            data_size: 0,
            code_data: None,
            reloc_32: None,
            symbols: None,
            line_debug_info: None,
        }
    }
}

impl fmt::Display for Hunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Hunk {{ type: {:?}, memory: {:?}, size: {} bytes }}",
            self.hunk_type, self.mem_type, self.data_size
        )
    }
}

pub struct HunkParser;

impl HunkParser {
    pub fn parse_file<P: AsRef<Path>>(filename: P) -> io::Result<Vec<Hunk>> {
        let file = File::open(filename)?;
        let mut reader = BufReader::new(file);
        Self::parse_hunks(&mut reader)
    }

    pub fn parse_hunks<R: Read + Seek>(reader: &mut R) -> io::Result<Vec<Hunk>> {
        Self::validate_hunk_header(reader)?;
        let (hunk_count, _hunk_sizes) = Self::read_hunk_table(reader)?;
        let mut hunks = Vec::with_capacity(hunk_count);

        for _ in 0..hunk_count {
            hunks.push(Self::parse_hunk(reader)?);
        }

        Ok(hunks)
    }

    fn validate_hunk_header<R: Read>(reader: &mut R) -> io::Result<()> {
        let hunk_header = Self::read_u32(reader)?;
        if hunk_header != HUNK_HEADER {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid HUNK_HEADER"));
        }
        Self::read_u32(reader)?; // Skip header/string section
        Ok(())
    }

    fn read_hunk_table<R: Read>(reader: &mut R) -> io::Result<(usize, Vec<u32>)> {
        let table_size = Self::read_u32(reader)?;
        let first_hunk = Self::read_u32(reader)?;
        let last_hunk = Self::read_u32(reader)?;

        if last_hunk < first_hunk {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Last hunk index is less than first hunk index",
            ));
        }

        let hunk_count = (last_hunk - first_hunk + 1) as usize;

        let hunk_sizes = (0..hunk_count)
            .map(|_| Self::read_u32(reader))
            .collect::<io::Result<Vec<_>>>()?;

        Ok((hunk_count, hunk_sizes))
    }

    fn parse_hunk<R: Read + Seek>(reader: &mut R) -> io::Result<Hunk> {
        let mut hunk = Hunk::default();

        loop {
            let hunk_type = Self::read_u32(reader)?;
            match hunk_type {
                HUNK_CODE => Self::parse_code_or_data(HunkType::Code, &mut hunk, reader)?,
                HUNK_DATA => Self::parse_code_or_data(HunkType::Data, &mut hunk, reader)?,
                HUNK_BSS => Self::parse_bss(&mut hunk, reader)?,
                HUNK_RELOC32 => Self::parse_reloc32(&mut hunk, reader)?,
                HUNK_SYMBOL => Self::parse_symbols(&mut hunk, reader)?,
                HUNK_DEBUG => Self::parse_debug(&mut hunk, reader)?,
                HUNK_END => return Ok(hunk),
                _ => Self::skip_hunk(reader, hunk_type)?,
            }
        }
    }

    fn parse_code_or_data<R: Read>(
        hunk_type: HunkType,
        hunk: &mut Hunk,
        reader: &mut R,
    ) -> io::Result<()> {
        let (size, mem_type) = Self::get_size_type(Self::read_u32(reader)?);
        let mut code_data = vec![0; size];
        reader.read_exact(&mut code_data)?;

        hunk.hunk_type = hunk_type;
        hunk.mem_type = mem_type;
        hunk.data_size = size;
        hunk.code_data = Some(code_data);
        Ok(())
    }

    fn parse_bss<R: Read>(hunk: &mut Hunk, reader: &mut R) -> io::Result<()> {
        let (size, mem_type) = Self::get_size_type(Self::read_u32(reader)?);
        hunk.hunk_type = HunkType::Bss;
        hunk.mem_type = mem_type;
        hunk.data_size = size;
        Ok(())
    }

    fn parse_reloc32<R: Read>(hunk: &mut Hunk, reader: &mut R) -> io::Result<()> {
        let mut relocs = Vec::new();
        loop {
            let count = Self::read_u32(reader)? as usize;
            if count == 0 {
                break;
            }
            let target = Self::read_u32(reader)? as usize;
            let offsets = (0..count)
                .map(|_| Self::read_u32(reader))
                .collect::<io::Result<Vec<_>>>()?;
            relocs.push(RelocInfo32 { target, offsets });
        }
        hunk.reloc_32 = Some(relocs);
        Ok(())
    }

    fn parse_symbols<R: Read>(hunk: &mut Hunk, reader: &mut R) -> io::Result<()> {
        let mut symbols = Vec::new();
        loop {
            let num_longs = Self::read_u32(reader)?;
            if num_longs == 0 {
                break;
            }
            let name = Self::read_name(reader, num_longs)?;
            let offset = Self::read_u32(reader)?;
            symbols.push(Symbol { name, offset });
        }
        if !symbols.is_empty() {
            symbols.sort_by_key(|s| s.offset);
            hunk.symbols = Some(symbols);
        }
        Ok(())
    }

    fn parse_debug<R: Read + Seek>(hunk: &mut Hunk, reader: &mut R) -> io::Result<()> {
        let num_longs = Self::read_u32(reader)? - 2;
        let base_offset = Self::read_u32(reader)?;
        let debug_tag = Self::read_u32(reader)?;

        if debug_tag != DEBUG_LINE {
            reader.seek(SeekFrom::Current((num_longs * 4) as i64))?;
            return Ok(());
        }

        let source_file = Self::fill_debug_info(base_offset, num_longs, reader)?;
        hunk.line_debug_info
            .get_or_insert_with(Vec::new)
            .push(source_file);
        Ok(())
    }

    fn fill_debug_info<R: Read>(
        base_offset: u32,
        num_longs: u32,
        reader: &mut R,
    ) -> io::Result<SourceFile> {
        let num_name_longs = Self::read_u32(reader)?;
        let name = Self::read_name(reader, num_name_longs)?;
        let num_lines = (num_longs - num_name_longs - 1) / 2;
        let lines = (0..num_lines)
            .map(|_| {
                let line_no = Self::read_u32(reader)? & 0xffffff;
                let offset = Self::read_u32(reader)?;
                Ok(SourceLine {
                    line: line_no,
                    offset: base_offset + offset,
                })
            })
            .collect::<io::Result<Vec<_>>>()?;

        Ok(SourceFile {
            name,
            base_offset,
            lines,
        })
    }

    fn skip_hunk<R: Read + Seek>(reader: &mut R, hunk_type: u32) -> io::Result<()> {
        println!("Skipping unknown hunk type: {:#x}", hunk_type);
        let seek_offset = Self::read_u32(reader)? as i64;
        reader.seek(SeekFrom::Current(seek_offset * 4))?;
        Ok(())
    }

    fn get_size_type(t: u32) -> (usize, MemoryType) {
        let size = (t & 0x0fffffff) * 4;
        let mem_type = match t & 0xf0000000 {
            HUNKF_CHIP => MemoryType::Chip,
            HUNKF_FAST => MemoryType::Fast,
            _ => MemoryType::Any,
        };
        (size as usize, mem_type)
    }

    fn read_name<R: Read>(reader: &mut R, num_longs: u32) -> io::Result<String> {
        let len = num_longs as usize * 4;
        let mut buffer = vec![0u8; len];
        reader.read_exact(&mut buffer)?;
        let end = buffer.iter().position(|&x| x == 0).unwrap_or(buffer.len());
        Ok(String::from_utf8_lossy(&buffer[..end]).into_owned())
    }

    fn read_u32<R: Read>(reader: &mut R) -> io::Result<u32> {
        let mut buffer = [0u8; 4];
        reader.read_exact(&mut buffer)?;
        Ok(u32::from_be_bytes(buffer))
    }
}
