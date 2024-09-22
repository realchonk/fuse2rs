use std::ffi::CString;
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
