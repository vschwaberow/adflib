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
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

pub mod adf_blk;
pub mod adf_disk;
pub mod adf_err;
pub mod adf_file;
pub mod adf_raw;
pub mod adf_str;

#[cfg(test)]
mod adf_tests {
    use crate::adf_blk::BootBlock;

    /// It creates a new `BootBlock` struct, and then asserts that the values of the fields are what we
    /// expect
    #[test]
    fn test_bootblock() {
        let adf_boot = BootBlock {
            dostype: [0; 4],
            checksum: 0,
            rootblock: 0,
            data: [0; 500 + 512],
        };
        assert_eq!(adf_boot.dostype, [b'D', b'O', b'S', b' ']);
        assert_eq!(adf_boot.checksum, 0);
        assert_eq!(adf_boot.rootblock, 0);
        assert_eq!(adf_boot.data.len(), 1012);
    }

    #[test]
    fn adf_rootblock() {}
}
