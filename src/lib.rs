//! A Rust library that allows you to combine separate files into one, contiguously.
//!
//! This crate offers two functions for combining files: [`threaded`] and [`single`], and utilities in [`os_impl`].
//!
//! The multithreaded version should only be used when either:
//! 1. There is a large enough number of files, or;
//! 2. The total file size is large enough.
//!
//! This is because of the added cost of creating the threads and managing their inputs and outputs.
//!
//! # feature flags
//! - `tracing` -- enables logging with a tracing subscriber
//! - `cli` -- is required for compiling the example cli app

#[cfg(feature = "tracing")]
use std::time::Instant;
use std::{
    fs::File,
    io::{self, BufRead},
    path::{Path, PathBuf},
    sync::Arc,
};

pub mod os_impl;
// we can use both impls of `write_all_at` since we do not use the file cursor
use os_impl::write_all_at;

mod log;
use log::{debug, info};

/// Parameters for use with [`threaded`].
///
/// # Examples
///
/// ```
/// # use combinefiles::Options;
/// // the fields are public
/// let a = Options {
///     max_threads: 10,
///     buf_size: 8196,
/// };
///
/// // using the default bufsize
/// let b = Options::threads(10);
///
/// // the default bufsize is 8196 so this is true.
/// assert_eq!(a, b);
///
/// // use all default values
/// Options::default();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Options {
    /// The maximum number of threads to be used in combining the files.
    ///
    /// The actual number of threads is limited to `files.len()`.
    pub max_threads: u32,

    /// The size of the internal buffer of [`io::BufWriter`] to use when copying.
    pub buf_size: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_threads: 8,
            buf_size: 8196,
        }
    }
}

impl Options {
    /// Specify the max number of threads to use with the default bufsize.
    #[must_use]
    pub fn threads(max_threads: u32) -> Self {
        Self {
            max_threads,
            ..Self::default()
        }
    }
}

/// Combine a list of files, in order, to one file using multiple threads.
///
/// `files` and `sizes` must be the same length and be in the same order.
///
/// # Examples
///
/// ```no_run
/// # use std::path::PathBuf;
/// # use combinefiles::{os_impl::file_size, Options};
/// // these will be combined into one file contiguously
/// // so it is important that it's in order
/// let files: Vec<PathBuf> = ["a-1.zip", "a-2.zip", "a-3.zip"].iter().map(PathBuf::from).collect();
/// let sizes: Vec<u64> = files.iter().map(|f| file_size(f)).collect();
/// // combine with a max of 10 threads
/// combinefiles::threaded(files, sizes, "a.zip", Options::threads(10));
/// ```
///
/// # Errors
///
/// See [`std::io::Error`].
///
/// # Panics
///
/// Fails if `files.len()` is greater than [`u32::MAX`].
pub fn threaded(
    files: Vec<PathBuf>,
    sizes: Vec<u64>,
    output: impl AsRef<Path>,
    options: Options,
) -> io::Result<()> {
    #[cfg(feature = "tracing")]
    let start = Instant::now();

    let final_file = {
        let f = File::create_new(output)?;
        Arc::new(f)
    };

    let Options {
        max_threads,
        buf_size,
    } = options;

    let files_len = files.len();
    let threads = max_threads.min(files_len.try_into().expect("not that many files"));
    info!("combining {files_len} files with {max_threads} ({threads}) threads");

    // (path, offset)
    let (tx, rx) = crossbeam_channel::bounded(threads as usize);

    std::thread::spawn(move || {
        for (n, file) in files.into_iter().enumerate() {
            let offset = sizes.iter().take(n).sum();
            debug!("sent ({file:?}, {offset})");
            tx.send((file, offset)).expect("channel cannot be closed");
        }
        drop(tx);
    });

    let mut thread_handles = Vec::with_capacity(threads as usize);
    for i in 0..threads {
        let rx = rx.clone();
        let final_file = final_file.clone();
        thread_handles.push(std::thread::spawn(move || -> io::Result<()> {
            while let Ok((path, initial_offset)) = rx.recv() {
                info!("[thread{i}] combining {path:?} at offset {initial_offset}");

                let mut offset = initial_offset;
                let mut file = io::BufReader::with_capacity(buf_size, File::open(&path)?);
                loop {
                    let buf = file.fill_buf()?;
                    let len = buf.len();
                    if len == 0 {
                        info!("[thread{i}] done with {path:?}");
                        break;
                    }
                    write_all_at(&final_file, buf, &mut offset)?;
                    file.consume(len);
                }

                info!("copied {} bytes from {path:?}", offset - initial_offset);
            }
            Ok(())
        }));
    }

    for handle in thread_handles {
        handle.join().unwrap()?;
    }

    info!("combined {} files in {:?}", files_len, start.elapsed());

    Ok(())
}

/// Combine a list of files, in order, to one file.
///
/// `files` and `sizes` must be the same length and be in the same order.
///
/// # Examples
///
/// ```no_run
/// # use std::path::PathBuf;
/// # use combinefiles::os_impl::file_size;
/// // these will be combined into one file contiguously
/// // so it is important that it's in order
/// let files: Vec<PathBuf> = ["a-1.zip", "a-2.zip", "a-3.zip"].iter().map(PathBuf::from).collect();
/// let sizes: Vec<u64> = files.iter().map(|f| file_size(f)).collect();
/// // combine into "a.zip"
/// combinefiles::single(&files, &sizes, "a.zip");
/// ```
/// # Errors
///
/// See [`std::io::Error`].
pub fn single(files: &[PathBuf], sizes: &[u64], output: impl AsRef<Path>) -> io::Result<()> {
    let mut final_file = File::create_new(output)?;

    let len = files.len();
    info!("combining {len} files");

    #[cfg(feature = "tracing")]
    let start = Instant::now();

    for (n, path) in files.iter().enumerate() {
        info!("{}/{len}: combining {path:?} ({} bytes)", n + 1, sizes[n]);
        let mut file = File::open(path)?;
        io::copy(&mut file, &mut final_file)?;
    }

    info!("combined {} files in {:?}", files.len(), start.elapsed());

    Ok(())
}
