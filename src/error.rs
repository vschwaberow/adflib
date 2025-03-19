// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2023
// - Volker Schwaberow <volker@schwaberow.de>

use std::fmt;
use std::io;

/// Custom error type for ADF operations
#[derive(Debug)]
pub enum AdfError {
    /// Error when a file or directory is not found
    NotFound(String),

    /// Error when a directory is full and cannot accept more entries
    DirectoryFull(String),

    /// Error when a bitmap operation fails
    BitmapError(String),

    /// Error when a disk is full
    DiskFull(String),

    /// Error when a checksum validation fails
    ChecksumError(String),

    /// Error when a file or directory name is invalid
    InvalidName(String),

    /// Error when a file or directory already exists
    AlreadyExists(String),

    /// Error when a block is invalid or corrupted
    InvalidBlock(usize, String),

    /// Error when a disk format is unsupported or invalid
    InvalidDiskFormat(String),

    /// Error when a path is invalid
    InvalidPath(String),

    /// Error when a permission is denied
    PermissionDenied(String),

    /// Error when a directory is not empty
    DirectoryNotEmpty(String),

    /// Error when an I/O operation fails
    IoError(io::Error),

    /// Other errors
    Other(String),
}

impl fmt::Display for AdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdfError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AdfError::DirectoryFull(msg) => write!(f, "Directory full: {}", msg),
            AdfError::BitmapError(msg) => write!(f, "Bitmap error: {}", msg),
            AdfError::DiskFull(_) => write!(f, "Disk is full"),
            AdfError::ChecksumError(msg) => write!(f, "Checksum error: {}", msg),
            AdfError::InvalidName(msg) => write!(f, "Invalid name: {}", msg),
            AdfError::AlreadyExists(msg) => write!(f, "Already exists: {}", msg),
            AdfError::InvalidBlock(block, msg) => write!(f, "Invalid block {}: {}", block, msg),
            AdfError::InvalidDiskFormat(msg) => write!(f, "Invalid disk format: {}", msg),
            AdfError::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
            AdfError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            AdfError::DirectoryNotEmpty(msg) => write!(f, "Directory not empty: {}", msg),
            AdfError::IoError(err) => write!(f, "I/O error: {}", err),
            AdfError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for AdfError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AdfError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for AdfError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => AdfError::NotFound(err.to_string()),
            io::ErrorKind::PermissionDenied => AdfError::PermissionDenied(err.to_string()),
            io::ErrorKind::AlreadyExists => AdfError::AlreadyExists(err.to_string()),
            io::ErrorKind::InvalidInput => AdfError::InvalidPath(err.to_string()),
            _ => AdfError::IoError(err),
        }
    }
}

/// Result type for ADF operations
pub type Result<T> = std::result::Result<T, AdfError>;
