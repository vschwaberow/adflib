#[cfg(test)]
mod tests {
    use super::*;
    use crate::disk::{
        format_creation_date, load_adf_from_zip, DiskType, ADF, ADF_NUM_SECTORS, ADF_NUM_TRACKS,
        ADF_SECTOR_SIZE, ADF_TRACK_SIZE, ROOT_BLOCK,
    };
    use crate::dms::{DMSPackingMode, DMSReader};
    use std::io::{self, Cursor};
    use std::{
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };

    use zip::write::{ExtendedFileOptions, FileOptions};

    #[test]
    fn test_adf_creation() {
        let adf = ADF {
            data: vec![0; ADF_TRACK_SIZE * ADF_NUM_TRACKS],
            bitmap: vec![false; ADF_NUM_SECTORS],
        };
        assert_eq!(adf.data.len(), ADF_TRACK_SIZE * ADF_NUM_TRACKS);
    }

    #[test]
    fn test_adf_formatting() {
        let mut adf = ADF {
            data: vec![0; ADF_TRACK_SIZE * ADF_NUM_TRACKS],
            bitmap: vec![false; ADF_NUM_SECTORS],
        };
        adf.format(DiskType::OFS, "TestDisk").unwrap();

        let boot_block = adf.read_boot_block();
        assert_eq!(&boot_block[0..3], b"DOS");
        assert_eq!(boot_block[3], 0); // OFS

        let root_block = adf.read_sector(ROOT_BLOCK);
        assert_eq!(root_block[0], 2); // T_HEADER
        assert_eq!(root_block[ADF_SECTOR_SIZE - 4], 0); // OFS

        let name_len = root_block[ADF_SECTOR_SIZE - 80] as usize;
        let name = String::from_utf8_lossy(
            &root_block[ADF_SECTOR_SIZE - 79..ADF_SECTOR_SIZE - 79 + name_len],
        );
        assert_eq!(name, "TestDisk");
    }

    #[test]
    fn test_file_listing() {
        let mut adf = ADF {
            data: vec![0; ADF_TRACK_SIZE * ADF_NUM_TRACKS],
            bitmap: vec![false; ADF_NUM_SECTORS],
        };
        adf.format(DiskType::OFS, "TestDisk").unwrap();

        let files = adf.list_root_directory().unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_disk_information() {
        let mut adf = ADF {
            data: vec![0; ADF_TRACK_SIZE * ADF_NUM_TRACKS],
            bitmap: vec![false; ADF_NUM_SECTORS],
        };
        adf.format(DiskType::FFS, "TestDisk").unwrap();
        let info = adf.information().unwrap();
        assert_eq!(info.disk_name, "TestDisk");
        assert_eq!(info.disk_size, (ADF_TRACK_SIZE * ADF_NUM_TRACKS) as u32);
        assert!(format!("{:?}", info).contains(&format!(
            "Disk Size: {} bytes",
            ADF_TRACK_SIZE * ADF_NUM_TRACKS
        )));
        assert_eq!(info.heads, 2);
        assert_eq!(info.tracks, (ADF_NUM_TRACKS / 2) as u8);
        assert_eq!(
            info.sectors_per_track,
            (ADF_TRACK_SIZE / ADF_SECTOR_SIZE) as u8
        );
        assert!(format!("{:?}", info).contains(&format!(
            "Sectors per Track: {}",
            ADF_TRACK_SIZE / ADF_SECTOR_SIZE
        )));
        assert!(format!("{:?}", info).contains(&format!("Bytes per Sector: {}", ADF_SECTOR_SIZE)));
    }

    #[test]
    fn test_read_write_sector() {
        let mut adf = ADF {
            data: vec![0; ADF_TRACK_SIZE * ADF_NUM_TRACKS],
            bitmap: vec![false; ADF_NUM_SECTORS],
        };

        let test_data = [42u8; ADF_SECTOR_SIZE];
        adf.write_sector(10, &test_data).unwrap();

        let read_data = adf.read_sector(10);
        assert_eq!(read_data, &test_data[..]);
    }

    fn test_format_creation_time() {
        let adf = ADF {
            data: vec![0; ADF_TRACK_SIZE * ADF_NUM_TRACKS],
            bitmap: vec![false; ADF_NUM_SECTORS],
        };
        let time = SystemTime::now();
        let result = format_creation_date(time);
        assert_eq!(result, format_creation_date(time));
    }

    fn test_format_protection_flags() {
        let adf = ADF {
            data: vec![0; ADF_TRACK_SIZE * ADF_NUM_TRACKS],
            bitmap: vec![false; ADF_NUM_SECTORS],
        };
        let flags = 0b10101010;
        let result = adf.format_protection_flags(flags);
        assert_eq!(result, flags.to_string());
    }

    #[test]
    fn test_load_adf_from_zip() {
        let mut zip_buffer = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
            let options: FileOptions<ExtendedFileOptions> =
                FileOptions::default().compression_method(zip::CompressionMethod::Stored);
            zip.start_file("test.adf", options).unwrap();
            zip.write_all(&vec![0; ADF_TRACK_SIZE * ADF_NUM_TRACKS])
                .unwrap();
            zip.finish().unwrap();
        }

        let adf = load_adf_from_zip(&zip_buffer, "test.adf").unwrap();
        assert_eq!(adf.data.len(), ADF_TRACK_SIZE * ADF_NUM_TRACKS);
    }


    #[test]
    fn test_dms_header_reading() {
        let input = vec![
            b'D', b'M', b'S', b'!', b'P', b'R', b'O', b' ', 0, 0, 0, 1, // info_bits
            0, 0, 0, 2, 0, 0, 0, 79, 0, 0, 0, 3, 0, 0, 0, 4, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0,
            7, 0, 0, 0, 8, 0, 9, 0, 10, 0, 11, 0, 2, 0, 12,
        ];

        let reader = DMSReader::new(Cursor::new(input)).unwrap();
        let info = reader.info();

        assert_eq!(info.signature, "DMS!");
        assert_eq!(info.header_type, "PRO ");
        assert_eq!(info.info_bits, 1);
        assert_eq!(info.date, 2);
        assert_eq!(info.low_track, 0);
        assert_eq!(info.high_track, 79);
        assert_eq!(info.packed_size, 3);
        assert_eq!(info.unpacked_size, 4);
        assert!(matches!(info.compression_mode, DMSPackingMode::Quick));
    }

    #[test]
    fn test_dms_none_mode() {
        let input = vec![
            b'D', b'M', b'S', b'!', b'P', b'R', b'O', b' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 79, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, b'T', b'R', 0, 0, 0, 0, 0, 5, 0, 0, 0, 5, 0, 0, 0, 0, 0,
            0, 0, 0, 1, 2, 3, 4, 5,
        ];

        let mut reader = DMSReader::new(Cursor::new(input)).unwrap();
        let track_data = reader.read_track().unwrap();
        assert_eq!(track_data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_dms_simple_mode() {
        let input = vec![
            b'D', b'M', b'S', b'!', b'P', b'R', b'O', b' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 79, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, b'T', b'R', 0, 0, 0, 0, 0, 7, 0, 0, 0, 7, 0, 1, 0, 0, 0,
            0, 0, 0, 1, 2, 0x90, 3, 65, 3, 4,
        ];

        let mut reader = DMSReader::new(Cursor::new(input)).unwrap();
        let track_data = reader.read_track().unwrap();
        assert_eq!(track_data, vec![1, 2, 65, 65, 65, 3, 4]);
    }

    #[test]
    fn test_dms_quick_mode() {
        let input = vec![
            b'D', b'M', b'S', b'!', b'P', b'R', b'O', b' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 79, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 2, 0, 0, b'T', b'R', 0, 0, 0, 0, 0x2C, 0x60, 0, 0, 0x2C, 0x60, 0,
            2, 0, 0, 0, 0, 0, 0, 0b10101010, 0b10101010, 0b10101010, 0b10101010, 0b10101010,
            0b10101010, 0b10101010, 0b10101010,
        ];

        let mut reader = DMSReader::new(Cursor::new(input)).unwrap();
        let track_data = reader.read_track().unwrap();
        assert_eq!(track_data.len(), 11360);
        assert_eq!(&track_data[0..4], &[0xAA, 0xAA, 0xAA, 0xAA]);
    }

    #[test]
    fn test_unsupported_packing_mode() {
        let input = vec![
            b'D', b'M', b'S', b'!', b'P', b'R', b'O', b' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 79, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 3, 0, 0, b'T', b'R', 0, 0, 0, 0, 0, 5, 0, 0, 0, 5, 0, 3, 0, 0, 0,
            0, 0, 0, 1, 2, 3, 4, 5,
        ];

        let mut reader = DMSReader::new(Cursor::new(input)).unwrap();
        let result = reader.read_track();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Unsupported);
    }

    #[test]
    fn test_invalid_track_header() {
        let input = vec![
            b'D', b'M', b'S', b'!', b'P', b'R', b'O', b' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 79, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, b'X', b'X', 0, 0, 0, 0, 0, 5, 0, 0, 0, 5, 0, 0, 0, 0, 0,
            0, 0, 0, 1, 2, 3, 4, 5,
        ];

        let mut reader = DMSReader::new(Cursor::new(input)).unwrap();
        let result = reader.read_track();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);

    fn create_test_adf() -> ADF {
        let mut adf = ADF::new(ADF_NUM_SECTORS, ADF_SECTOR_SIZE);
        adf.format(DiskType::FFS, "TestDisk").unwrap();
        adf
    }

    #[test]
    fn test_create_directory() {
        let mut adf = create_test_adf();

        adf.create_directory("TestDir").unwrap();

        let root_files = adf.list_root_directory().unwrap();
        let test_dir = root_files.iter().find(|f| f.name == "TestDir" && f.is_dir);
        assert!(test_dir.is_some(), "TestDir not found in root directory");
    }

    #[test]
    fn test_rename_directory() {
        let mut adf = create_test_adf();

        adf.create_directory("OldDir").unwrap();
        adf.rename_directory("OldDir", "NewDir").unwrap();

        let root_files = adf.list_root_directory().unwrap();
        let old_dir = root_files.iter().find(|f| f.name == "OldDir");
        let new_dir = root_files.iter().find(|f| f.name == "NewDir" && f.is_dir);

        assert!(old_dir.is_none(), "OldDir still exists");
        assert!(new_dir.is_some(), "NewDir not found in root directory");
    }

    #[test]
    fn test_delete_directory() {
        let mut adf = create_test_adf();

        adf.create_directory("DeleteMe").unwrap();
        adf.delete_directory("DeleteMe").unwrap();

        let root_files = adf.list_root_directory().unwrap();
        let deleted_dir = root_files.iter().find(|f| f.name == "DeleteMe");

        assert!(deleted_dir.is_none(), "DeleteMe directory still exists");
    }

    #[test]
    fn test_delete_non_empty_directory() {
        let mut adf = create_test_adf();

        adf.create_directory("ParentDir").unwrap();
        adf.create_directory("ParentDir/ChildDir").unwrap();

        let result = adf.delete_directory("ParentDir");

        assert!(result.is_err(), "Deleting non-empty directory should fail");
    }

    #[test]
    fn test_rename_non_existent_directory() {
        let mut adf = create_test_adf();

        let result = adf.rename_directory("NonExistent", "NewName");

        assert!(
            result.is_err(),
            "Renaming non-existent directory should fail"
        );

    }
}
