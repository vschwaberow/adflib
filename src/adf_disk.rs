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

use crate::adf_blk::*;
use crate::adf_err::*;
use crate::adf_raw::*;
use crate::adf_str::*;

/// It reads the boot block, copies the boot code into it, and writes it back
///
/// Arguments:
///
/// * `vol`: a pointer to a Volume structure
/// * `code`: The boot code to be written to the disk.
///
/// Returns:
///
/// The return code of the function.
pub fn adf_install_boot_block(vol: &mut Volume, code: &mut [u8; 1024]) -> i32 {
    let mut _i: i32;
    let mut boot: BootBlock = BootBlock::new();

    let device_type = vol.device.device_type;

    if device_type != DEVICETYPE_FLOPDD && device_type != DEVICETYPE_FLOPHD {
        return RC_ERROR;
    }

    if adf_read_boot_block(vol, &mut boot) != RC_OK {
        return RC_ERROR;
    }

    boot.rootblock = 880;
    for i in 0..1024 - 12 {
        /* bootcode */
        boot.data[i] = code[i + 12];
    }

    if adf_write_boot_block(vol, &mut boot) != RC_OK {
        return RC_ERROR;
    }

    vol.boot_code = true;

    RC_OK
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
