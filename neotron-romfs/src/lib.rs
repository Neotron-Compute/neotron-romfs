//! Library for creating or parsing a Neotron ROM Filing System (ROMFS) image
//!
//! To view the contents of a ROMFS, use a for loop:
//!
//! ```rust
//! fn process_rom(data: &[u8]) -> Result<(), neotron_romfs::Error> {
//!     let romfs = neotron_romfs::RomFs::new(data)?;
//!     for entry in romfs {
//!         if let Ok(entry) = entry {
//!            println!("{} is {} bytes", entry.metadata.file_name, entry.metadata.file_size);
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! To open a specific file, use [`RomFs::find`]:
//!
//! ```rust
//! fn process_rom(romfs: &neotron_romfs::RomFs) {
//!     if let Some(entry) = romfs.find("HELLO.ELF") {
//!         let data: &[u8] = entry.contents;
//!     }
//! }
//! ```

#![no_std]

/// The ways in which this module can fail
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Error {
    /// We didn't see the magic number at the start of the ROMFS
    InvalidMagicHeader,
    /// The given size was not the same length that the header reported
    WrongSize,
    /// Did not recognise the version
    UnknownVersion,
    /// Buffer was too small to hold ROMFS image
    BufferTooSmall,
    /// Filename was too long (we have a 14 byte maximum)
    FilenameTooLong,
    /// A filename wasn't valid UTF-8
    NonUnicodeFilename,
    /// There was an error writing to a sink
    SinkError,
}

/// The different image formats we support
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FormatVersion {
    /// The first version
    Version100 = 1,
}

/// Represents a ROM Filing System (ROMFS), as backed by a byte slice in memory.
pub struct RomFs<'a> {
    contents: &'a [u8],
}

impl<'a> RomFs<'a> {
    /// Mount a ROMFS using a given block of RAM
    pub fn new(contents: &'a [u8]) -> Result<RomFs<'a>, Error> {
        let (header, remainder) = Header::from_bytes(contents)?;
        if contents.len() != header.total_size as usize {
            return Err(Error::WrongSize);
        }
        Ok(RomFs {
            contents: remainder,
        })
    }

    /// Find a file in the ROMFS, by name.
    pub fn find(&self, file_name: &str) -> Option<Entry<&str, &[u8]>> {
        self.into_iter()
            .flatten()
            .find(|e| e.metadata.file_name == file_name)
    }

    /// Construct a ROMFS into the given buffer.
    ///
    /// Tells you how many bytes it used of the given buffer.
    ///
    /// The buffer must be large enough otherwise an error is returned - see
    /// [`Self::size_required`] to calculate the size of buffer required.
    pub fn construct<S, T>(mut buffer: &mut [u8], entries: &[Entry<S, T>]) -> Result<usize, Error>
    where
        S: AsRef<str>,
        T: AsRef<[u8]>,
    {
        let total_size = Self::size_required(entries);
        if buffer.len() < total_size {
            return Err(Error::BufferTooSmall);
        }
        let used = Self::construct_into(&mut buffer, entries)?;
        Ok(used)
    }

    /// Construct a ROMFS into the given embedded-io byte sink.
    ///
    /// Tells you how many bytes it wrote to the given buffer.
    pub fn construct_into<S, T, SINK>(
        buffer: &mut SINK,
        entries: &[Entry<S, T>],
    ) -> Result<usize, Error>
    where
        S: AsRef<str>,
        T: AsRef<[u8]>,
        SINK: embedded_io::Write,
    {
        let total_size = Self::size_required(entries);
        let file_header = Header {
            format_version: FormatVersion::Version100,
            total_size: total_size as u32,
        };
        let mut used = file_header.write_into(buffer)?;
        for entry in entries.iter() {
            used += entry.metadata.write_into(buffer)?;
            let contents: &[u8] = entry.contents.as_ref();
            buffer.write_all(contents).map_err(|_| Error::SinkError)?;
            used += contents.len();
        }

        assert_eq!(used, total_size);

        Ok(total_size)
    }

    /// Tells you how many bytes you need to make a ROMFS from these entries.
    pub fn size_required<S, T>(entries: &[Entry<S, T>]) -> usize
    where
        S: AsRef<str>,
        T: AsRef<[u8]>,
    {
        let mut total_size: usize = Header::FIXED_SIZE;
        for entry in entries.iter() {
            total_size += EntryMetadata::<S>::SIZE;
            let contents: &[u8] = entry.contents.as_ref();
            total_size += contents.len();
        }
        total_size
    }
}

impl<'a> IntoIterator for RomFs<'a> {
    type Item = Result<Entry<&'a str, &'a [u8]>, Error>;

    type IntoIter = RomFsEntryIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        RomFsEntryIter {
            contents: self.contents,
        }
    }
}

impl<'a> IntoIterator for &'a RomFs<'a> {
    type Item = Result<Entry<&'a str, &'a [u8]>, Error>;

    type IntoIter = RomFsEntryIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        RomFsEntryIter {
            contents: self.contents,
        }
    }
}

/// An iterator for working through the entries in a ROMFS
pub struct RomFsEntryIter<'a> {
    contents: &'a [u8],
}

impl<'a> Iterator for RomFsEntryIter<'a> {
    type Item = Result<Entry<&'a str, &'a [u8]>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.contents.is_empty() {
            return None;
        }
        match EntryMetadata::<&str>::from_bytes(self.contents) {
            Ok((hdr, remainder)) => {
                if hdr.file_size as usize > remainder.len() {
                    // stop if we run out of data
                    return None;
                }
                let (contents, remainder) = remainder.split_at(hdr.file_size as usize);
                self.contents = remainder;
                Some(Ok(Entry {
                    metadata: hdr,
                    contents,
                }))
            }
            Err(e) => {
                // stop the iteration
                self.contents = &[];
                Some(Err(e))
            }
        }
    }
}

/// Found at the start of the ROMFS image
///
/// In flash we have 8 bytes of magic number, four bytes of version and four
/// bytes of length.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Header {
    pub format_version: FormatVersion,
    pub total_size: u32,
}

impl Header {
    const MAGIC_VALUE: [u8; 8] = *b"NeoROMFS";
    const FORMAT_V100: [u8; 4] = [0x00, 0x01, 0x00, 0x00];
    const FIXED_SIZE: usize = 8 + 4 + 4;

    /// Parse a header from raw bytes.
    fn from_bytes(data: &[u8]) -> Result<(Header, &[u8]), Error> {
        let Some(magic_value) = data.get(0..8) else {
            return Err(Error::BufferTooSmall);
        };
        if magic_value != Self::MAGIC_VALUE {
            return Err(Error::InvalidMagicHeader);
        }
        let Some(format_version) = data.get(8..12) else {
            return Err(Error::BufferTooSmall);
        };
        if format_version == Self::FORMAT_V100 {
            let Some(total_size) = data.get(12..16) else {
                return Err(Error::UnknownVersion);
            };
            let total_size: [u8; 4] = total_size.try_into().unwrap();
            let total_size = u32::from_be_bytes(total_size);
            let hdr = Header {
                format_version: FormatVersion::Version100,
                total_size,
            };
            Ok((hdr, &data[16..]))
        } else {
            Err(Error::UnknownVersion)
        }
    }

    /// Write the header to the given buffer
    fn write_into<SINK>(&self, buffer: &mut SINK) -> Result<usize, Error>
    where
        SINK: embedded_io::Write,
    {
        buffer
            .write_all(&Self::MAGIC_VALUE)
            .map_err(|_| Error::SinkError)?;
        buffer
            .write_all(match self.format_version {
                FormatVersion::Version100 => &Self::FORMAT_V100,
            })
            .map_err(|_| Error::SinkError)?;
        let size_bytes = self.total_size.to_be_bytes();
        buffer
            .write_all(&size_bytes)
            .map_err(|_| Error::SinkError)?;
        Ok(Header::FIXED_SIZE)
    }
}

/// An entry in the ROMFS, including its contents.
#[derive(Debug, PartialEq, Eq)]
pub struct Entry<S, T>
where
    S: AsRef<str>,
    T: AsRef<[u8]>,
{
    /// Metadata for this entry.
    pub metadata: EntryMetadata<S>,
    /// The file data for this entry.
    ///
    /// Call `contents.as_ref()` to get the contents as a byte slice
    /// (`&[u8]`).
    pub contents: T,
}

/// Metadata for an entry in the ROMFS.
///
/// Occupies [`Self::SIZE`] bytes of ROM when encoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryMetadata<S>
where
    S: AsRef<str>,
{
    /// The file name for this entry.
    ///
    /// Call `file_name.as_ref()` to get the contents as a string slice
    /// (`&str`).
    pub file_name: S,
    /// The creation time, of the file associated with this entry.
    pub ctime: neotron_api::file::Time,
    /// The size, in bytes, of the file associated with this entry.
    pub file_size: u32,
}

impl<S> EntryMetadata<S>
where
    S: AsRef<str>,
{
    const FILENAME_SIZE: usize = 14;
    const FILENAME_OFFSET: usize = 0;
    const FILESIZE_SIZE: usize = 4;
    const FILESIZE_OFFSET: usize = Self::FILENAME_OFFSET + Self::FILENAME_SIZE;
    const TIMESTAMP_SIZE: usize = 6;
    const TIMESTAMP_OFFSET: usize = Self::FILESIZE_OFFSET + Self::FILESIZE_SIZE;

    /// The size of this metadata, in bytes, when encoded as bytes.
    pub const SIZE: usize = Self::TIMESTAMP_OFFSET + Self::TIMESTAMP_SIZE;

    /// Parse out some entry metadata from raw bytes.
    ///
    /// We assume the bytes are correctly formatted - we can't check much here
    /// other than the filename being valid UTF-8, or that too few bytes were
    /// given.
    ///
    /// Returns the entry and the remaining unused bytes, or an error.
    fn from_bytes(data: &[u8]) -> Result<(EntryMetadata<&str>, &[u8]), Error> {
        let Some(file_name) =
            data.get(Self::FILENAME_OFFSET..Self::FILENAME_OFFSET + Self::FILENAME_SIZE)
        else {
            return Err(Error::BufferTooSmall);
        };
        let Ok(file_name) = core::str::from_utf8(file_name) else {
            return Err(Error::NonUnicodeFilename);
        };
        let file_name = file_name.trim_end_matches('\0');
        let ctime = neotron_api::file::Time {
            year_since_1970: *data
                .get(Self::TIMESTAMP_OFFSET)
                .ok_or(Error::BufferTooSmall)?,
            zero_indexed_month: *data
                .get(Self::TIMESTAMP_OFFSET + 1)
                .ok_or(Error::BufferTooSmall)?,
            zero_indexed_day: *data
                .get(Self::TIMESTAMP_OFFSET + 2)
                .ok_or(Error::BufferTooSmall)?,
            hours: *data
                .get(Self::TIMESTAMP_OFFSET + 3)
                .ok_or(Error::BufferTooSmall)?,
            minutes: *data
                .get(Self::TIMESTAMP_OFFSET + 4)
                .ok_or(Error::BufferTooSmall)?,
            seconds: *data
                .get(Self::TIMESTAMP_OFFSET + 5)
                .ok_or(Error::BufferTooSmall)?,
        };
        let Some(file_size) =
            data.get(Self::FILESIZE_OFFSET..Self::FILESIZE_OFFSET + Self::FILESIZE_SIZE)
        else {
            return Err(Error::BufferTooSmall);
        };
        // We got four bytes above so this can't fail
        let file_size: [u8; 4] = file_size.try_into().unwrap();
        let file_size = u32::from_be_bytes(file_size);
        let stored_entry = EntryMetadata {
            file_name,
            file_size,
            ctime,
        };
        Ok((stored_entry, &data[Self::SIZE..]))
    }

    /// Write this entry to the sink.
    ///
    /// Returns the number of bytes written.
    fn write_into<SINK>(&self, sink: &mut SINK) -> Result<usize, Error>
    where
        SINK: embedded_io::Write,
    {
        // check the file name isn't too long
        let file_name = self.file_name.as_ref();
        let file_name_len = file_name.len();
        let Some(padding_length) = Self::FILENAME_SIZE.checked_sub(file_name_len) else {
            return Err(Error::FilenameTooLong);
        };
        // copy file name with null padding
        sink.write_all(file_name.as_bytes())
            .map_err(|_| Error::SinkError)?;
        for _ in 0..padding_length {
            sink.write_all(&[0u8]).map_err(|_| Error::SinkError)?;
        }
        // copy file size
        let file_size = self.file_size.to_be_bytes();
        sink.write_all(&file_size).map_err(|_| Error::SinkError)?;
        // copy timestamp
        sink.write_all(&[self.ctime.year_since_1970])
            .map_err(|_| Error::SinkError)?;
        sink.write_all(&[self.ctime.zero_indexed_month])
            .map_err(|_| Error::SinkError)?;
        sink.write_all(&[self.ctime.zero_indexed_day])
            .map_err(|_| Error::SinkError)?;
        sink.write_all(&[self.ctime.hours])
            .map_err(|_| Error::SinkError)?;
        sink.write_all(&[self.ctime.minutes])
            .map_err(|_| Error::SinkError)?;
        sink.write_all(&[self.ctime.seconds])
            .map_err(|_| Error::SinkError)?;

        Ok(Self::SIZE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_empty() {
        #[rustfmt::skip]
        let data = [
            // Magic number
            0x4e, 0x65, 0x6f, 0x52, 0x4f, 0x4d, 0x46, 0x53,
            // Version
            0x00, 0x01, 0x00, 0x00,
            // Total size
            0x00, 0x00, 0x00, 0x10,
        ];
        let romfs = RomFs::new(&data).unwrap();
        let mut i = romfs.into_iter();
        assert!(i.next().is_none());
    }

    #[test]
    fn decode_bad_len() {
        #[rustfmt::skip]
        let data = [
            // Magic number
            0x4e, 0x65, 0x6f, 0x52, 0x4f, 0x4d, 0x46, 0x53,
            // Version
            0x00, 0x01, 0x00, 0x00,
            // Total size
            0x00, 0x00, 0x00, 0x0F,
        ];
        assert!(RomFs::new(&data).is_err());
    }

    #[test]
    fn decode_bad_magic() {
        #[rustfmt::skip]
        let data = [
            // Magic number
            0x4e, 0x65, 0x6f, 0x52, 0x4f, 0x4d, 0x46, 0x54,
            // Version
            0x00, 0x01, 0x00, 0x00,
            // Total size
            0x00, 0x00, 0x00, 0x10,
        ];
        assert!(RomFs::new(&data).is_err());
    }

    #[test]
    fn decode_bad_version() {
        #[rustfmt::skip]
        let data = [
            // Magic number
            0x4e, 0x65, 0x6f, 0x52, 0x4f, 0x4d, 0x46, 0x53,
            // Version
            0x00, 0x01, 0x00, 0x01,
            // Total size
            0x00, 0x00, 0x00, 0x10,
        ];
        assert!(RomFs::new(&data).is_err());
    }

    #[test]
    fn decode_one_file() {
        #[rustfmt::skip]
        let data = [
            // Magic number
            0x4e, 0x65, 0x6f, 0x52, 0x4f, 0x4d, 0x46, 0x53,
            // Version
            0x00, 0x01, 0x00, 0x00,
            // Total size
            0x00, 0x00, 0x00, 0x2C,
            // File Name
            0x52, 0x45, 0x41, 0x44, 0x4d, 0x45, 0x2e, 0x54, 0x58, 0x54, 0x00, 0x00, 0x00, 0x00,
            // File size
            0x00, 0x00, 0x00, 0x04,
            // Timestamp (2023-11-12T20:05:16)
            0x35, 0x0A, 0x0B, 0x14, 0x05, 0x10,
            // Contents
            0x12, 0x34, 0x56, 0x78,
        ];
        let romfs = RomFs::new(&data).unwrap();
        let mut i = romfs.into_iter();
        let first_item = i.next().unwrap().unwrap();
        assert_eq!(first_item.metadata.file_name, "README.TXT");
        assert_eq!(first_item.contents.len(), 4);
        assert_eq!(first_item.contents, &[0x12, 0x34, 0x56, 0x78]);
        assert_eq!(
            first_item.metadata.ctime,
            neotron_api::file::Time {
                year_since_1970: 53,
                zero_indexed_month: 10,
                zero_indexed_day: 11,
                hours: 20,
                minutes: 5,
                seconds: 16
            }
        );
        assert!(i.next().is_none());
    }

    #[test]
    fn decode_two_files() {
        #[rustfmt::skip]
        let data = [
            // Magic number
            0x4e, 0x65, 0x6f, 0x52, 0x4f, 0x4d, 0x46, 0x53,
            // Version
            0x00, 0x01, 0x00, 0x00,
            // Total size
            0x00, 0x00, 0x00, 0x47,
            // File Name
            b'R', b'E', b'A', b'D', b'M', b'E', b'.', b'T', b'X', b'T', 0x00, 0x00, 0x00, 0x00,
            // File size
            0x00, 0x00, 0x00, 0x04,
            // Timestamp (2023-11-12T20:05:16)
            0x35, 0x0A, 0x0B, 0x14, 0x05, 0x10,
            // Contents
            0x12, 0x34, 0x56, 0x78,
            // File Name
            b'H', b'E', b'L', b'L', b'O', b'.', b'D', b'O', b'C', 0x00, 0x00, 0x00, 0x00, 0x00,
            // File size
            0x00, 0x00, 0x00, 0x03,
            // Timestamp (2023-11-12T20:05:17)
            0x35, 0x0A, 0x0B, 0x14, 0x05, 0x11,
            // Contents
            0xAB, 0xCD, 0xEF,
        ];
        let romfs = RomFs::new(&data).unwrap();
        let mut i = romfs.into_iter();
        let first_item = i.next().unwrap().unwrap();
        assert_eq!(first_item.metadata.file_name, "README.TXT");
        assert_eq!(first_item.contents.len(), 4);
        assert_eq!(first_item.contents, &[0x12, 0x34, 0x56, 0x78]);
        assert_eq!(
            first_item.metadata.ctime,
            neotron_api::file::Time {
                year_since_1970: 53,
                zero_indexed_month: 10,
                zero_indexed_day: 11,
                hours: 20,
                minutes: 5,
                seconds: 16
            }
        );
        let second_item = i.next().unwrap().unwrap();
        assert_eq!(second_item.metadata.file_name, "HELLO.DOC");
        assert_eq!(second_item.contents.len(), 3);
        assert_eq!(second_item.contents, &[0xAB, 0xCD, 0xEF]);
        assert_eq!(
            second_item.metadata.ctime,
            neotron_api::file::Time {
                year_since_1970: 53,
                zero_indexed_month: 10,
                zero_indexed_day: 11,
                hours: 20,
                minutes: 5,
                seconds: 17
            }
        );
        assert!(i.next().is_none());
    }
}

// End of file
