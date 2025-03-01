// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2023
// - Volker Schwaberow <volker@schwaberow.de>

pub const ADF_TRACK_SIZE: usize = 11 * ADF_SECTOR_SIZE;
pub const ADF_NUM_TRACKS: usize = 80 * 2;
pub const ROOT_BLOCK: usize = 880;
pub const ADF_SECTOR_SIZE: usize = 512;
pub const ADF_NUM_SECTORS: usize = 1760;
pub const DIR_ENTRY_SIZE: usize = 4;
pub const DIR_ENTRIES_START: usize = 24;
pub const DIR_ENTRIES_END: usize = 51;
pub const FILE_HEADER_SIZE: usize = 24;
pub const FILE_NAME_MAX_LEN: usize = 30;
pub const FILE_HEADER_BLOCK_OFFSET: usize = 16;
pub const FILE_SIZE_OFFSET: usize = 4;
pub const FILE_NAME_LEN_OFFSET: usize = 432;
pub const FILE_NAME_OFFSET: usize = 433;
pub const FILE_PROTECTION_OFFSET: usize = 436;
pub const FILE_DAYS_OFFSET: usize = 440;
pub const FILE_MINS_OFFSET: usize = 444;
pub const FILE_TICKS_OFFSET: usize = 448;
pub const ROOT_BLOCK_SIZE_OFFSET: usize = 12;
pub const ROOT_BLOCK_NAME_LEN_OFFSET: usize = ADF_SECTOR_SIZE - 80;
pub const ROOT_BLOCK_NAME_OFFSET: usize = ADF_SECTOR_SIZE - 79;
pub const ROOT_BLOCK_DAYS_OFFSET: usize = ADF_SECTOR_SIZE - 92;
pub const ROOT_BLOCK_MINS_OFFSET: usize = ADF_SECTOR_SIZE - 88;
pub const ROOT_BLOCK_TICKS_OFFSET: usize = ADF_SECTOR_SIZE - 84;
pub const ROOT_BLOCK_TYPE_OFFSET: usize = ADF_SECTOR_SIZE - 4;
pub const ROOT_BLOCK_HASH_TABLE_SIZE: u16 = 72;
pub const ROOT_BLOCK_HASH_TABLE_OFFSET: usize = 12;
pub const ROOT_BLOCK_RESERVED_BLOCKS_OFFSET: usize = 128;
pub const ROOT_BLOCK_RESERVED_BLOCKS_END: usize = 135;
pub const ROOT_BLOCK_CREATION_DATE_OFFSET: usize = 16;
pub const ROOT_BLOCK_CREATION_DATE_END: usize = 19;
pub const BITMAP_BLOCK_FLAG_OFFSET: usize = 0;
pub const BITMAP_BLOCK_CHECKSUM_OFFSET: usize = 4;
pub const BITMAP_BLOCK_SIZE: usize = 220;
pub const BITMAP_BLOCK_START: usize = 2;
pub const BITMAP_BLOCK_END: usize = ADF_NUM_SECTORS;
pub const SECONDS_PER_DAY: u64 = 86400;
pub const SECONDS_PER_HOUR: u64 = 3600;
pub const SECONDS_PER_MINUTE: u64 = 60;

