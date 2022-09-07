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



pub const DEVICETYPE_FLOPDD: u32 = 1;
pub const DEVICETYPE_FLOPHD: u32 = 2;
pub const DEVICETYPE_HARDDISK: u32 = 3;
pub const DEVICETYPE_HARDFILE: u32 = 4;

pub struct Volume {
    pub device: Device,
    pub firstblock: u32,
    pub lastblock: u32,
    pub rootblock: u32,
    pub dos_type: u8,
    pub boot_code: bool,
    pub read_only: bool,
    pub datablocksize: u16,
    pub blocksize: u16,
    pub volume_name: Vec<String>,
    pub mounted: bool,
    pub dirty: bool,
    pub bitmap_size: u32,
    pub bitmap_blocks: u32,
    pub current_dir_ptr: u32,
}

pub struct Device {
    pub device_type: u32,
    pub read_only: bool,
    pub dirty: bool,
    pub size: u32,
    pub num_volumes: u32,
    pub volume: Vec<Volume>,
    pub cyls: u32,
    pub heads: u32,
    pub secs: u32,
    pub is_native: bool,
    pub native_device: Vec<u8>,
}

pub struct Amigafile {
    pub volume: Vec<Volume>,
    pub file_header: Vec<Fileheaderblock>,
    pub current_data: Vec<u8>,
    pub current_ext: Vec<Fileextblock>,
    pub num_datablock: u32,
    pub current_dataptr: u32,
    pub pos: u32,
    pub pos_in_datablock: u32,
    pub pos_in_extblock: u32,
    pub eof: bool,
    pub write_mode: bool,
}

pub struct NativeDevice {
    // TODO: implement file descriptor
}

pub struct NativeFunctions {

    is_native: bool,
}

impl NativeFunctions {
    pub fn new() -> NativeFunctions {
        NativeFunctions {
            is_native: false,
        }
    }

    pub fn init_device(device: &Device, name: &str, ro: bool) {


    }

}