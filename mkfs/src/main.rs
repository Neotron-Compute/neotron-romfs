//! Neotron mkfs utility - makes Neotron ROM Filesytems
//!
//! * Takes a series of command-line arguments, which should each be a path to file.
//! * Writes a valid ROMFS image to `stdout`, containing all those files.

use chrono::{Datelike, Timelike};
use std::io::Write;

/// Entry point to the binary
fn main() -> Result<(), std::io::Error> {
    let mut entries = Vec::new();
    for file_path in std::env::args_os().skip(1) {
        let file_path: &std::path::Path = file_path.as_ref();
        eprintln!("Loading {}", file_path.display());
        let contents = std::fs::read(file_path)?;
        let Some(file_name) = file_path.file_name() else {
            panic!("Path {} doesn't have a filename?", file_path.display());
        };
        let Some(file_name_str) = file_name.to_str() else {
            panic!("Path {} has a non UTF-8 filename", file_path.display());
        };
        let metadata = std::fs::metadata(file_path)?;
        let ctime = metadata.created().unwrap_or(std::time::SystemTime::now());
        let ctime = chrono::DateTime::<chrono::Utc>::from(ctime);
        let entry = neotron_romfs::Entry {
            metadata: neotron_romfs::EntryMetadata {
                file_name: file_name_str.to_owned(),
                ctime: neotron_api::file::Time {
                    year_since_1970: (ctime.year() - 1970) as u8,
                    zero_indexed_month: ctime.month0() as u8,
                    zero_indexed_day: ctime.day0() as u8,
                    hours: ctime.hour() as u8,
                    minutes: ctime.minute() as u8,
                    seconds: ctime.second() as u8,
                },
                file_size: contents.len() as u32,
            },
            contents,
        };
        entries.push(entry);
    }

    // make this plenty big enough
    let mut output: Vec<u8> = Vec::new();
    match neotron_romfs::RomFs::construct_into(&mut output, &entries) {
        Ok(_n) => {
            let mut out = std::io::stdout();
            out.write_all(&output)?;
        }
        Err(e) => {
            panic!("Failed to build ROMFS: {:?}", e);
        }
    }

    Ok(())
}
