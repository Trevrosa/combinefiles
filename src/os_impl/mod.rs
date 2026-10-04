//! Operating-system dependent functions.
//!
//! Note that the [`write_all_at`] implementations differ between Unix and Windows systems:
//!
//! On Unix, the file cursor is not affected by the function.
//!
//! On Windows, the file cursor **is** affected by the function; it is changed to the end of the write.

use std::{fs::File, io, path::Path};

#[allow(unused, reason = "for docs")]
use std::io::Write;

/// Get the size (in bytes) of a file from its metadata.
///
/// # Panics
/// Fails if the metadata could not be read.
#[cfg(windows)]
#[must_use]
pub fn file_size(p: &Path) -> u64 {
    use std::os::windows::fs::MetadataExt;
    p.metadata().map(|m| m.file_size()).unwrap()
}

/// Get the size (in bytes) of a file from its metadata.
///
/// # Panics
/// Fails if the metadata could not be read.
#[cfg(unix)]
#[must_use]
pub fn file_size(p: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    p.metadata().map(|m| m.size()).unwrap()
}

/// Attempts to write an entire buffer starting from a given offset.
///
/// The offset is relative to the start of the file and thus independent from the current cursor.
///
/// The current file cursor is not affected by this function.
///
/// When writing beyond the end of the file, the file is appropriately extended and the intermediate bytes are set to zero.
///
/// Note that similar to [`File::write`], it is not an error to return a short write.
///
/// # Errors
/// This function will return the first error of non-[`io::ErrorKind::Interrupted`] kind that a write returns.
#[cfg(unix)]
pub fn write_all_at(writer: &File, buf: &[u8], offset: &mut u64) -> io::Result<()> {
    use std::os::unix::fs::FileExt;
    writer.write_all_at(buf, *offset)?;
    *offset += buf.len() as u64;
    Ok(())
}

/// Attempts to write an entire buffer starting from a given offset.
///
/// The offset is relative to the start of the file and thus independent from the current cursor.
///
/// The current cursor **is** affected by this function, it is set to the end of the write.
///
/// When writing beyond the end of the file, the file is appropriately extended and the intermediate bytes are set to zero.
///
/// Note that similar to [`File::write`], it is not an error to return a short write.
///
/// # Errors
/// This function will return the first error of non-[`io::ErrorKind::Interrupted`] kind that a write returns.
#[cfg(windows)]
pub fn write_all_at(writer: &File, mut buf: &[u8], offset: &mut u64) -> io::Result<()> {
    // Copies implementation from unix::fs::FileExt
    use io::ErrorKind::{Interrupted, WriteZero};
    use std::os::windows::fs::FileExt;
    while !buf.is_empty() {
        match writer.seek_write(buf, *offset) {
            Ok(0) => return Err(io::Error::new(WriteZero, "failed to write full buffer")),
            Ok(n) => {
                buf = &buf[n..];
                *offset += n as u64;
            }
            Err(ref e) if matches!(e.kind(), Interrupted) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
