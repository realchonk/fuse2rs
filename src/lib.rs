use std::path::Path;
use std::io::Result;
use std::time::SystemTime;

mod ll;

pub use crate::ll::DirFiller;

#[derive(Default)]
pub enum FileType {
    #[default]
    RegularFile,
    Directory,
    NamedPipe,
    Socket,
    CharDevice,
    BlockDevice,
    Symlink,
}

pub struct FileAttr {
    pub ino: u64,
    pub size: u64,
    pub blocks: u64,
    pub atime: SystemTime,
    pub mtime: SystemTime,
    pub ctime: SystemTime,
    pub btime: SystemTime,
    pub kind: FileType,
    pub perm: u16,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u32,
    pub blksize: u32,
    pub flags: u32,
    pub nlink: u32,
}

impl Default for FileAttr {
    fn default() -> Self {
	Self {
	    ino: 0,
	    size: 0,
	    blocks: 0,
	    atime: SystemTime::UNIX_EPOCH,
	    mtime: SystemTime::UNIX_EPOCH,
	    ctime: SystemTime::UNIX_EPOCH,
	    btime: SystemTime::UNIX_EPOCH,
	    kind: FileType::default(),
	    perm: 0,
	    uid: 0,
	    gid: 0,
	    rdev: 0,
	    blksize: 512,
	    flags: 0,
	    nlink: 1,
	}
    }
}

pub trait Filesystem {
    fn getattr(&mut self, path: &Path) -> Result<FileAttr>;
    fn readdir(&mut self, path: &Path, off: u64, filler: &mut DirFiller) -> Result<()>;
    fn read(&mut self, path: &Path, off: u64, buf: &mut [u8]) -> Result<usize>;
    fn open(&mut self, path: &Path) -> Result<()>;
}


pub fn mount(mp: &Path, fs: impl Filesystem + 'static) {
    crate::ll::xmount(mp, Box::new(fs));
}
