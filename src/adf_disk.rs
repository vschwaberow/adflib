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


struct Volume {
    dev: Device,
    boot_code: bool,
}

struct Device {
    dev_type: DeviceType,
}

enum DeviceType {
    FLOPDD,
    FLOPHD,
}

struct BootBlock {
    root_block: u32,
    data: [u8; 1024-12],
}

fn install_boot_block(vol: &mut Volume, code: &[u8]) -> Result<(), Error> {
    if vol.dev.dev_type != DEVICETYPE_FLOPDD && vol.dev.dev_type != DEVICETYPE_FLOPHD {
        return Err(Error::InvalidDeviceType);
    }

    let mut boot = match read_boot_block(vol) {
        Ok(b) => b,
        Err(e) => return Err(e),
    };

    boot.root_block = 880;
    boot.data.copy_from_slice(&code[12..]);

    match write_boot_block(vol, &boot) {
        Ok(_) => (),
        Err(e) => return Err(e),
    }

    vol.boot_code = true;

    Ok(())
}


/// If the number of sectors is greater than or equal to zero and less than or equal to the last block
/// minus the first block, then return true.
///
/// Arguments:
///
/// * `volume`: The volume to check against.
/// * `num_sectors`: The number of sectors to read.
///
/// Returns:
///
/// A boolean value.
fn is_sect_num_valid(volume: &Volume, num_sectors: i32) -> bool {
    return 0 <= num_sectors && num_sectors <= volume.lastblock - volume.firstblock;
}

/// `unmount` unmounts a volume
///
/// Arguments:
///
/// * `volume`: The volume to unmount.
fn unmount(volume: &Volume) {
    todo!()
}
