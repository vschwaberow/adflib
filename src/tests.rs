#[cfg(test)]
mod tests {
    use super::*;
    use crate::disk::{
        format_creation_date, load_adf_from_zip, DiskType, ADF, ADF_NUM_SECTORS, ADF_NUM_TRACKS,
        ADF_SECTOR_SIZE, ADF_TRACK_SIZE, ROOT_BLOCK,
    };
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
}
