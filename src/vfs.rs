//! Access to the [Virtual File System (VFS) layer](http://doc.riot-os.org/group__sys__vfs.html)
//!
//! This abstraction tries not to be smart about modes -- a [File] opened with RDONLY will still
//! have a write method, and because file operations are generally fallible, writes will just fail.
//!
//! ## Panics
//!
//! This module violently asserts that file names are UTF-8 encoded (a condition easily satisified
//! if only ASCII file names are used).
//!
//! ## Incomplete
//!
//! So far, only a subset of VFS is implemented; in particular, the file system is read-only.

use core::marker::PhantomData;
use core::mem::MaybeUninit;

use riot_sys::libc;

use crate::error::{NegativeErrorExt, NumericError};
use crate::helpers::{PointerToCStr, SliceToCStr};

/// A file handle
#[derive(Debug)]
pub struct File {
    // Nonnegative, actually -- but as long as NumericError isn't known-negative, this doesn't help
    // with returning results.
    fileno: libc::c_int,
    // Sending file descriptors around is currently possible in RIOT, but discouraged
    _not_send_sync: PhantomData<*const ()>,
}

/// Results of a file stat operation
#[derive(Debug)]
pub struct Stat(riot_sys::stat);

impl Stat {
    /// The current size of the file
    pub fn size(&self) -> usize {
        self.0.st_size as _
    }
}

/// Parameter for seeking in a file
///
/// It is analogous to [std::io::SeekFrom].
#[derive(Debug)]
pub enum SeekFrom {
    /// Seek to the given position from the start of the file
    Start(usize),
    /// Seek to the given position relative to the end of the file
    End(isize),
    /// Seek to the given position relative to the current cursor position
    Current(isize),
}

impl File {
    /// Open a file in read-only mode.
    pub fn open(path: &str) -> Result<Self, NumericError> {
        let fileno = unsafe {
            riot_sys::vfs_open(
                path as *const str as *const libc::c_char,
                riot_sys::O_RDONLY as _,
                0,
            )
        }
        .negative_to_error()?;
        Ok(File {
            fileno,
            _not_send_sync: PhantomData,
        })
    }

    /// Obtain metadata of the file.
    pub fn stat(&self) -> Result<Stat, NumericError> {
        let mut stat = MaybeUninit::uninit();
        (unsafe { riot_sys::vfs_fstat(self.fileno, stat.as_mut_ptr()) }).negative_to_error()?;
        let stat = unsafe { stat.assume_init() };
        Ok(Stat(stat))
    }

    /// Read into the given buffer from the current cursor position in the file, and advance the
    /// cursor by the read length, which is also returned.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, NumericError> {
        (unsafe {
            riot_sys::vfs_read(
                self.fileno,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len() as _,
            )
        })
        .negative_to_error()
        .map(|len| len as _)
    }

    /// Move the file cursor to the indicated position.
    pub fn seek(&mut self, pos: SeekFrom) -> Result<usize, NumericError> {
        let (off, whence) = match pos {
            SeekFrom::Start(i) => (i as _, riot_sys::SEEK_SET as _),
            SeekFrom::Current(i) => (i as _, riot_sys::SEEK_CUR as _),
            SeekFrom::End(i) => (i as _, riot_sys::SEEK_END as _),
        };
        (unsafe { riot_sys::vfs_lseek(self.fileno, off, whence) })
            .negative_to_error()
            .map(|r| r as _)
    }
}

impl Drop for File {
    fn drop(&mut self) {
        unsafe { riot_sys::vfs_close(self.fileno) };
    }
}


/// A directory in the file system
///
/// The directory can be iterated over, producing directory entries one by one.
#[repr(transparent)]
pub struct Dir(riot_sys::vfs_DIR, core::marker::PhantomPinned);

impl Dir {
    pub fn open(dir: &str) -> Result<Self, NumericError> {
        let mut dirp = MaybeUninit::uninit();
        (unsafe {
            riot_sys::vfs_opendir(dirp.as_mut_ptr(), dir as *const str as *const libc::c_char)
        })
        .negative_to_error()?;
        let dirp = unsafe { dirp.assume_init() };
        Ok(Dir(dirp, core::marker::PhantomPinned))
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        unsafe { riot_sys::vfs_closedir(&mut self.0) };
    }
}

impl Iterator for Dir {
    type Item = Dirent;

    fn next(&mut self) -> Option<Dirent> {
        let mut ent = MaybeUninit::uninit();
        let ret = (unsafe { riot_sys::vfs_readdir(&mut self.0, ent.as_mut_ptr()) })
            .negative_to_error()
            .ok()?;
        if ret > 0 {
            Some(Dirent(unsafe { ent.assume_init() }))
        } else {
            None
        }
    }
}

/// Directory entry inside a file
///
/// The entry primarily indicates the file's name.
pub struct Dirent(riot_sys::vfs_dirent_t);

impl Dirent {
    /// Name of the file
    ///
    /// This will panic if the file name is not encoded in UTF-8.
    pub fn name(&self) -> &str {
        let mut name = self
            .0
            .d_name
            .to_cstr()
            // *We* could continue, but it's way more likely to be an error
            .expect("File name does not have a trailing null character")
            .to_str()
            .expect("File name not UTF-8 encoded");

        // Workaround for https://github.com/RIOT-OS/RIOT/issues/14635
        while name.starts_with("/") {
            name = &name[1..];
        }

        name
    }
}

/// A mount point, represented (and made un-unmountable) by its root directory
pub struct Mount<'a>(&'a mut riot_sys::vfs_DIR);

/// Lending iterator over all mount points
///
/// Note that while looking like an iterator, this does not actually implement Iterator -- it
/// can't, for not all the items it produces necessarily live long enough. (It could if there were
/// an `fdup` for directories, but then again that'd be wasteful for the typical case where the
/// user doesn't need the iterator's long lifetime).
///
/// While `LendingIterator` is not in the core library, this just implements something sufficiently
/// similar in the style of the
/// [StreamingIterator](https://docs.rs/streaming-iterator/latest/streaming_iterator/) (thus
/// avoiding GATs).
pub struct MountIter {
    dir: MaybeUninit<riot_sys::vfs_DIR>,
    _phantom: core::marker::PhantomPinned,
}

impl MountIter {
    pub fn next(&mut self) -> Option<Mount<'_>> {
        // unsafe: Our dir is always either zeroed or managed by mount_dirs
        if unsafe { riot_sys::vfs_iterate_mount_dirs(self.dir.as_mut_ptr()) } {
            // unsafe: API says there's something initialized in there (and the lifetime is
            // justified from locking self which owns the dir)
            Some(Mount(unsafe { self.dir.assume_init_mut() }))
        } else {
            // Go back to starting condition because there's no guarantee this won't be called
            // after the last element. This restores safe order, and also contains the information
            // Drop needs to decide whether or not something is in here that needs to be closed.
            self.dir = MaybeUninit::zeroed();
            None
        }
    }

    fn is_zeroed(&self) -> bool {
        // unsafe: Type has the right size and u8 seems like the best way to test for zeroness
        (unsafe {
            core::slice::from_raw_parts(
                self.dir.as_ptr() as *const u8,
                core::mem::size_of::<riot_sys::vfs_DIR>(),
            )
        }) == &[0; core::mem::size_of::<riot_sys::vfs_DIR>()]
    }
}

impl Drop for MountIter {
    fn drop(&mut self) {
        if !self.is_zeroed() {
            // unsafe: API function used as documented in vfs_iterate_mount_dirs
            unsafe { riot_sys::vfs_closedir(self.dir.as_mut_ptr()) };
        }
    }
}

impl<'a> Mount<'a> {
    /// List all mount points
    #[doc(alias = "vfs_iterate_mount_dirs")]
    pub fn all() -> MountIter {
        MountIter {
            dir: MaybeUninit::zeroed(),
            _phantom: core::marker::PhantomPinned,
        }
    }

    /// Use the mount point as a directory iterator
    ///
    /// Note that reading its entries mutates the `Mount` instance as the opened directory is
    /// internal to it; a second call to this function may produce an empty iterator (just like
    /// attempting to read entries from an already exhausted [Dir] does); this may change if VFS's
    /// directories gain rewind support.
    pub fn root_dir(&mut self) -> &'a mut Dir {
        // unsafe: Legitimized by the Dir being transparent, and by Dir not doing anything that'd
        // invalidate the dir's openness as long as it's not owned.
        unsafe { &mut *(self.0 as *mut _ as *mut _) }
    }

    pub fn mount_point(&self) -> &'a str {
        // FIXME: Docs say to treat as opaque
        unsafe { (*self.0.mp).mount_point.to_lifetimed_cstr() }
            .expect("Mount point is NULL")
            .to_str()
            .expect("Mount point not UTF-8 encoded")
    }
}
