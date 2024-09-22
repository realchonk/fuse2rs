use std::{ffi::CString, io::Error};
use std::path::Path;
use std::io::Result;
use std::time::SystemTime;
use libc::{gid_t, mode_t, uid_t};

mod ll;

pub use crate::ll::DirFiller;

pub struct Request {
	pub uid: uid_t,
	pub gid: gid_t,
	pub umask: mode_t,
}

pub struct FileInfo {
	pub flags: i32,
	pub fh: u64,
	pub direct_io: bool,
	pub keep_cache: bool,
	pub flush: bool,
	pub nonseekable: bool,
}

pub trait Filesystem {
	fn getattr(
		&mut self,
		_req: &Request,
		path: &Path,
	) -> Result<FileAttr>;

	fn readdir(
		&mut self,
		_req: &Request,
		path: &Path,
		off: u64,
		filler: &mut DirFiller,
		_info: &FileInfo,
	) -> Result<()>;

	fn read(
		&mut self,
		_req: &Request,
		path: &Path,
		off: u64,
		buf: &mut [u8],
		_info: &FileInfo,
	) -> Result<usize>;

	// OPTIONAL

	// TODO: KernelConfig
	fn init(&mut self, _req: &Request) {}
	fn destroy(&mut self) {}

	fn open(
		&mut self,
		_req: &Request,
		path: &Path,
		_info: &mut FileInfo,
	) -> Result<()> {
		let _ = path;
		Ok(())
	}

	fn opendir(
		&mut self,
		_req: &Request,
		path: &Path,
		_info: &mut FileInfo,
	) -> Result<()> {
		let _ = path;
		Ok(())
	}

	fn statfs(
		&mut self,
		_req: &Request,
		path: &Path,
	) -> Result<Statfs> {
		let _ = path;
		Ok(Statfs::default())
	}

	fn readlink(
		&mut self,
		_req: &Request,
		path: &Path,
		buf: &mut [u8],
	) -> Result<usize> {
		let _ = (path, buf);
		Err(Error::from_raw_os_error(libc::ENOSYS))
	}
}

#[derive(Debug, Default, Clone, Copy)]
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

#[derive(Debug, Default, Clone)]
pub struct Statfs {
	pub bsize: u32,
	pub frsize: u32,
	pub blocks: u64,
	pub bfree: u64,
	pub bavail: u64,
	pub files: u64,
	pub ffree: u64,
	pub favail: u64,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum MountOption {
	Foreground,
	Debug,
	AllowOther,
	DefaultPermissions,
	KernelCache,
	Ro,
	Rw,
	Atime,
	NoAtime,
	Dev,
	NoDev,
	Suid,
	NoSuid,
	Exec,
	NoExec,
	Sync,
	Async,
	UseIno,
	ReaddirIno,
	HardRemove,
	Uid(u32),
	Gid(u32),
	Umask(u16),
	Custom(CString),
}

impl MountOption {
	fn into_cstring(self) -> CString {
		match self {
			Self::Foreground => c"-f".into(),
			Self::Debug => c"-d".into(),
			Self::AllowOther => c"-oallow_other".into(),
			Self::DefaultPermissions => c"-odefault_permissions".into(),
			Self::KernelCache => c"-okernel_cache".into(),
			Self::Ro => c"-oro".into(),
			Self::Rw => c"-orw".into(),
			Self::Atime => c"-oatime".into(),
			Self::NoAtime => c"-onoatime".into(),
			Self::Dev => c"-odev".into(),
			Self::NoDev => c"-onodev".into(),
			Self::Exec => c"-oexec".into(),
			Self::NoExec => c"-onoexec".into(),
			Self::Suid => c"-osuid".into(),
			Self::NoSuid => c"-onosuid".into(),
			Self::Sync => c"-osync".into(),
			Self::Async => c"-oasync".into(),
			Self::UseIno => c"-ouse_ino".into(),
			Self::ReaddirIno => c"-oreaddir_ino".into(),
			Self::HardRemove => c"-ohard_remove".into(),
			Self::Uid(uid) => CString::new(format!("-ouid={uid}")).unwrap(),
			Self::Gid(gid) => CString::new(format!("-ogid={gid}")).unwrap(),
			Self::Umask(mask) => CString::new(format!("-oumask={mask:o}")).unwrap(),
			Self::Custom(c) => c,
		}
	}
}

pub fn mount(mp: &Path, fs: impl Filesystem + 'static, opts: Vec<MountOption>) -> Result<()> {
	let opts = opts
		.into_iter()
		.map(|opt| opt.into_cstring())
		.collect();
	crate::ll::xmount(mp, Box::new(fs), opts)
}
