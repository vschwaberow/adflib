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
pub const MAXCOMMENTLENGTH: usize = 79;
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
    pub dostype: [u8; 4],
    pub checksum: u32,
    pub rootblock: i32,
    pub data: [u8; 500+512]
}

pub struct RootBlock {
    pub amiga_type: i32,
    pub headerkey: i32,
    pub highseq: i32,
    pub hashtablesize: i32,
    pub firstdata: i32,
    pub checksum: u32,
    pub hashtable: [i32; HT_SIZE],
    pub bmflag: i32,
    pub bmpages: [i32; BM_SIZE],
    pub bmext: i32,
    pub cdays: i32,
    pub cmins: i32,
    pub cticks: i32,
    pub namelength: i8,
    pub diskname: [i8; MAXNAMELENGTH],
    pub r2: [i8; 8],
    pub days: i32,
    pub mins: i32,
    pub ticks: i32,
    pub codays: i32,
    pub comins: i32,
    pub coticks: i32,
    pub nextsamehash: i32,
    pub parent: i32,
    pub extension: i32,
    pub sectype: i32,
}

pub struct Fileheaderblock {
    pub amiga_type: i32,		
    pub headerkey: i32,	
    pub highseq: i32,	
    pub datasize: i32,	
    pub firstdata: i32,
    pub checksum: u32,
    pub datablocks: [i32; MAX_DATABLOCK],
    pub r1: i32,
    pub r2: i32,
    pub access: i32,	
    pub bytesize: u32,
    pub commlen: u8,
    pub comment: [u8; MAXCOMMENTLENGTH+1],
    pub days: i32,
    pub mins: i32,
    pub ticks: i32,
    pub namelen: u8,
    pub filename: [u8; MAXNAMELENGTH+1],
    pub real: i32,		
    pub nextlink: i32,	
    pub nextsamehash: i32,	
    pub parent: i32,		
    pub extension: i32,
    pub sectype: i32,	
}

pub struct Directoryblock {
    pub amiga_type: i32,
    pub headerkey: i32,
    pub highseq: i32,
    pub hashtabsize: i32,
    pub r1: i32,
    pub hashtable: [i32; HT_SIZE],
    pub r2: [i32; 2],
    pub access: i32,
    pub r4: i32,
    pub commlen: u8,
    pub comment: [u8; MAXCOMMENTLENGTH+1],
    pub r5: [u8; 91-(MAXCOMMENTLENGTH+1)],
    pub days: i32,
    pub mins: i32,
    pub ticks: i32,
    pub namelen: u8,
    pub dirname: [u8; MAXNAMELENGTH+1],
    pub r6: i32,
    pub real: i32,
    pub nextlink: i32,
    pub r7: [i32; 5],
    pub nextsamehash: i32,
    pub parent: i32,
    pub extension: i32,
    pub sectype: i32
}

pub struct OFSDatablock {
    pub amiga_type: i32,
    pub headerkey: i32,
    pub seqnum: i32,
    pub datasize: i32,
    pub nextdata: i32,
    pub checksum: u32,
    pub data: [u8; 488],
}

pub struct Bitmapblock {
    pub checksum: u32,
    pub map: [u32; 127]
}

pub struct Bitmapextblock {
    pub bmpages: [i32; 127],
    pub nextblock: i32
}

pub struct Linkblock {
    pub amiga_type: i32,      
    pub headerkey: i32, 
    pub r1: [i32; 3],
    pub checksum: u32,
    pub realname: [u8; 64],
    pub r2: [i32; 83],
    pub days: i32,     
    pub mins: i32,
    pub ticks: i32,
    pub namelen: u8,
    pub name: [u8; MAXNAMELENGTH + 1],
    pub r3: i32,
    pub realentry: i32,
    pub nextlink: i32,
    pub r4: [i32; 5],
    pub nextsamehash: i32,
    pub parent: i32,
    pub r5: i32,
    pub sectype: i32,  
}



