#![allow(dead_code)]

use std::borrow::Cow;

pub struct FujiFix;

impl FujiFix {
    pub fn apply<'a>(buffer: &'a [u8]) -> Cow<'a, [u8]> {
        if buffer.len() >= 4 && &buffer[..4] == b"FUJI" {
            Cow::Borrowed(&buffer[148..])
        } else {
            Cow::Borrowed(buffer)
        }
    }
}
