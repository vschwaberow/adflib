// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2023
// - Volker Schwaberow <volker@schwaberow.de>

// ADF Disk Structure Constants
pub const ADF_TRACK_SIZE: usize = 11 * ADF_SECTOR_SIZE; // Size of a track in bytes (11 sectors per track)
pub const ADF_NUM_TRACKS: usize = 80 * 2; // Total number of tracks (80 tracks, 2 sides)
pub const ROOT_BLOCK: usize = 880; // Block number of the root directory
pub const ADF_SECTOR_SIZE: usize = 512; // Size of a sector in bytes
pub const ADF_NUM_SECTORS: usize = 1760; // Total number of sectors on the disk

// Sector and Block Constants
pub const SECTOR_SIZE: usize = ADF_SECTOR_SIZE; // Alias for ADF_SECTOR_SIZE
pub const FIRST_DATA_BLOCK: usize = 2; // First data block index

// Directory Entry Constants
pub const DIR_ENTRY_SIZE: usize = 4; // Size of a directory entry in bytes
pub const DIR_ENTRIES_START: usize = 24; // First directory entry index
pub const DIR_ENTRIES_END: usize = 51; // Last directory entry index

// Directory Block Constants
pub const DIR_ENTRY_START_INDEX: usize = 24; // First directory entry index
pub const DIR_ENTRY_END_INDEX: usize = 51; // Last directory entry index
pub const DIR_PARENT_OFFSET: usize = 4; // Offset to the parent directory block pointer
pub const DIR_CREATION_DATE_OFFSET: usize = 420; // Offset to the creation date
pub const DIR_NAME_LEN_OFFSET: usize = FILE_NAME_LEN_OFFSET; // Offset to the directory name length
pub const DIR_NAME_OFFSET: usize = FILE_NAME_OFFSET; // Offset to the directory name
pub const DIR_DAYS_OFFSET: usize = FILE_DAYS_OFFSET; // Offset to the days component of creation date
pub const DIR_MINS_OFFSET: usize = FILE_MINS_OFFSET; // Offset to the minutes component of creation date
pub const DIR_TICKS_OFFSET: usize = FILE_TICKS_OFFSET; // Offset to the ticks component of creation date
pub const DIR_CHECKSUM_OFFSET: usize = 20; // Offset to start calculating checksum from
pub const DIR_CHECKSUM_LOCATION: usize = 0; // Location to store the checksum

// File Header Constants
pub const FILE_HEADER_SIZE: usize = 24; // Size of a file header in bytes
pub const FILE_NAME_MAX_LEN: usize = 30; // Maximum length of a file name
pub const FILE_HEADER_BLOCK_OFFSET: usize = 16; // Offset to the file header block pointer
pub const FILE_SIZE_OFFSET: usize = 4; // Offset to the file size in the header
pub const FILE_NAME_LEN_OFFSET: usize = 432; // Offset to the file name length
pub const FILE_NAME_OFFSET: usize = 433; // Offset to the file name
pub const FILE_PROTECTION_OFFSET: usize = 436; // Offset to the file protection flags
pub const FILE_DAYS_OFFSET: usize = 440; // Offset to the days component of creation date
pub const FILE_MINS_OFFSET: usize = 444; // Offset to the minutes component of creation date
pub const FILE_TICKS_OFFSET: usize = 448; // Offset to the ticks component of creation date

// Root Block Constants
pub const ROOT_BLOCK_SIZE_OFFSET: usize = 12; // Offset to the root block size
pub const ROOT_BLOCK_NAME_LEN_OFFSET: usize = ADF_SECTOR_SIZE - 80; // Offset to the disk name length
pub const ROOT_BLOCK_NAME_OFFSET: usize = ADF_SECTOR_SIZE - 79; // Offset to the disk name
pub const ROOT_BLOCK_DAYS_OFFSET: usize = ADF_SECTOR_SIZE - 92; // Offset to the days component of creation date
pub const ROOT_BLOCK_MINS_OFFSET: usize = ADF_SECTOR_SIZE - 88; // Offset to the minutes component of creation date
pub const ROOT_BLOCK_TICKS_OFFSET: usize = ADF_SECTOR_SIZE - 84; // Offset to the ticks component of creation date
pub const ROOT_BLOCK_TYPE_OFFSET: usize = ADF_SECTOR_SIZE - 4; // Offset to the root block type
pub const ROOT_BLOCK_HASH_TABLE_SIZE: u16 = 72; // Size of the hash table in the root block
pub const ROOT_BLOCK_HASH_TABLE_OFFSET: usize = 12; // Offset to the hash table in the root block
pub const ROOT_BLOCK_RESERVED_BLOCKS_OFFSET: usize = 128; // Offset to the reserved blocks
pub const ROOT_BLOCK_RESERVED_BLOCKS_END: usize = 135; // End offset of reserved blocks
pub const ROOT_BLOCK_CREATION_DATE_OFFSET: usize = 16; // Offset to the creation date
pub const ROOT_BLOCK_CREATION_DATE_END: usize = 19; // End offset of creation date
pub const ROOT_BLOCK_BITMAP_FLAG_OFFSET: usize = ADF_SECTOR_SIZE - 200; // Offset to the bitmap flag in root block
pub const ROOT_BLOCK_BITMAP_FLAG_VALUE: u8 = 0xFF; // Value of the bitmap flag
pub const ROOT_BLOCK_DISK_TYPE_OFFSET: usize = 432; // Offset to the disk type in the root block
pub const ROOT_BLOCK_HASH_TABLE_SIZE_OFFSET: usize = 12; // Offset to the hash table size
pub const ROOT_BLOCK_BITMAP_COUNT: usize = 25; // Number of bitmap blocks
pub const ROOT_BLOCK_BITMAP_POINTERS_OFFSET: usize = 24; // Offset to the bitmap block pointers

// Bitmap Block Constants
pub const BITMAP_BLOCK_FLAG_OFFSET: usize = 0; // Offset to the bitmap block flag
pub const BITMAP_BLOCK_CHECKSUM_OFFSET: usize = 4; // Offset to the bitmap block checksum
pub const BITMAP_BLOCK_SIZE: usize = 220; // Size of the bitmap block in bytes
pub const BITMAP_BLOCK_START: usize = 2; // First block in the bitmap
pub const BITMAP_BLOCK_END: usize = ADF_NUM_SECTORS; // Last block in the bitmap
pub const BITMAP_BLOCK_FIRST_BYTE: u8 = 0xF8; // First byte value in the bitmap block
pub const BITMAP_BLOCK: usize = 881; // Block number of the first bitmap block
pub const BITMAP_FLAG_OFFSET: usize = 4; // Offset to the bitmap flags
pub const BITMAP_VALID_OFFSET: usize = 8; // Offset to the bitmap valid flag
pub const BITMAP_CHECKSUM_OFFSET: usize = 12; // Offset to the bitmap checksum
pub const BITMAP_CHECKSUM_LOCATION: usize = 0; // Location of the bitmap checksum
pub const BITMAP_HEADER_OFFSET: usize = 0; // Offset to the bitmap header
pub const BITMAP_HEADER_VALUE: u8 = 0x02; // Value of the bitmap header

// Boot Block Constants
pub const BOOT_BLOCK_SIZE: usize = 2; // Size of the boot block in sectors
pub const BOOT_BLOCK_SIGNATURE_SIZE: usize = 4; // Size of the boot block signature
pub const BOOT_BLOCK_SIGNATURE: &[u8] = b"DOS\0"; // Boot block signature
pub const BOOT_BLOCK_FLAGS_OFFSET: usize = 3; // Offset to the boot block flags

// Time Constants
pub const SECONDS_PER_DAY: u64 = 86400; // Number of seconds in a day
pub const SECONDS_PER_HOUR: u64 = 3600; // Number of seconds in an hour
pub const SECONDS_PER_MINUTE: u64 = 60; // Number of seconds in a minute
pub const TICKS_PER_SECOND: u32 = 50; // Number of ticks in a second (Amiga time)

// Protection Flag Constants
pub const PROTECTION_FLAGS_MASK: u32 = 0xFF; // Mask for protection flags
pub const PROTECTION_FLAG_HIDDEN: u32 = 0x80; // Hidden flag (h)
pub const PROTECTION_FLAG_SCRIPT: u32 = 0x40; // Script flag (s)
pub const PROTECTION_FLAG_PURE: u32 = 0x20; // Pure flag (p)
pub const PROTECTION_FLAG_ARCHIVE: u32 = 0x10; // Archive flag (a)
pub const PROTECTION_FLAG_READ: u32 = 0x08; // Read flag (r)
pub const PROTECTION_FLAG_WRITE: u32 = 0x04; // Write flag (w)
pub const PROTECTION_FLAG_EXECUTE: u32 = 0x02; // Execute flag (e)
pub const PROTECTION_FLAG_DELETE: u32 = 0x01; // Delete flag (d)

// Block Type Constants
pub const BLOCK_TYPE_OFFSET: usize = 0; // Offset to the block type
pub const BLOCK_TYPE_FILE: u8 = 0; // File block type
pub const BLOCK_TYPE_DIRECTORY: u8 = 2; // Directory block type
pub const BLOCK_TYPE_FILE_HEADER: u8 = 2; // File header block type
pub const BLOCK_TYPE_FILE_LIST: u8 = 16; // File list block type
pub const BLOCK_TYPE_BITMAP: u8 = 0; // Bitmap block type

// Data Block Constants
pub const NEXT_BLOCK_OFFSET: usize = 0; // Offset to the next block pointer
pub const DATA_BLOCK_HEADER_SIZE: usize = 24; // Size of the data block header
pub const DATA_BLOCK_NEXT_OFFSET: usize = 0; // Offset to the next data block pointer
pub const DATA_BLOCK_DATA_OFFSET: usize = 24; // Offset to the data in a data block
pub const DATA_BLOCK_SIZE: usize = 488; // Size of the data in a data block
pub const FILE_LIST_NEXT_OFFSET: usize = 0; // Offset to the next file list block
pub const FILE_LIST_ENTRY_OFFSET: usize = 24; // Offset to the first file list entry
pub const FILE_LIST_ENTRIES_PER_BLOCK: usize = 72; // Number of entries per file list block

// Filesystem Type Constants
pub const FILESYSTEM_TYPE_OFS: u8 = 0; // Old File System type
pub const FILESYSTEM_TYPE_FFS: u8 = 1; // Fast File System type

// Name Constants
pub const MAX_NAME_LENGTH: usize = 30; // Maximum length of a name
