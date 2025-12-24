#![allow(dead_code)]

use crate::rawthumb::formats::format_processor::FormatPreprocessor;

pub struct FujiPreprocessor;

impl FujiPreprocessor {
    fn apply_fix<'a>(&self, buffer: &'a [u8]) -> &'a [u8] {
        if buffer.len() >= 4 && &buffer[..4] == b"FUJI" {
            &buffer[148..]
        } else {
            buffer
        }
    }
}

impl FormatPreprocessor for FujiPreprocessor {
    fn preprocess<'a>(&self, buffer: &'a [u8]) -> &'a [u8] {
        self.apply_fix(buffer)
    }
}
