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




// constant values for the file system
pub const FSMASK_FFS: u8 = 1;
pub const FSMASK_INTL: u8 = 2;
pub const FSMASK_DIRCACHE: u8 = 4;

// constant values for the block type   

pub const MAXNAMELENGTH: usize = 30;
pub const MAXCOMMANDLENGTH: usize = 79;
pub const HT_SIZE: usize = 72;
pub const BM_SIZE: usize = 25;
pub const MAX_DATABLOCK: usize = 72;
pub const BM_VALID: isize = -1;
pub const BM_INVALID: isize = 0;

pub fn is_ffs(c: u8) -> bool { (c & FSMASK_FFS) != 0 }
pub fn is_ofs(c: u8) -> bool { (c & FSMASK_FFS) == 0 }
pub fn is_intl(c: u8) -> bool { (c & FSMASK_INTL) != 0 }
pub fn is_dircache(c: u8) -> bool { (c & FSMASK_DIRCACHE) != 0 }


pub struct BootBlock {
    dostype: [char; 4],
    checksum: u32,
    rootblock: i32,
    data: [u8; 500+512]
}

pub struct RootBlock {
    amiga_type: i32,
    headerkey: i32,
    highseq: i32,
    hashtablesize: i32,
    firstdata: i32,
    checksum: u32,
    hashtable: [i32; HT_SIZE],
    bmflag: i32,
    bmpages: [i32; BM_SIZE],
    bmext: i32,
    cdays: i32,
    cmins: i32,
    cticks: i32,
    namelength: i8,
    diskname: [i8; MAXNAMELENGTH],
    r2: [i8; 8],
    days: i32,
    mins: i32,
    ticks: i32,
    codays: i32,
    comins: i32,
    coticks: i32,
    nextsamehash: i32,
    parent: i32,
    extension: i32,
    sectype: i32,
}