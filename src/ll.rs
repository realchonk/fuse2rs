use crate::{FileType, Filesystem};
use std::{
	iter::once,
	os::unix::ffi::OsStrExt,
	path::Path,
	ffi::*,
	time::SystemTime,
	io::{Result, Error},
};


#[allow(dead_code, unused_variables, non_camel_case_types, non_snake_case)]
mod fuse2 {
	include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub struct DirFiller {
	func: fuse2::fuse_fill_dir_t,
	data: *mut c_void,
}

impl DirFiller {
	pub fn push(&mut self, name: &CStr, /* TODO: off, st */) -> bool {
		match unsafe { self.func.unwrap()(self.data, name.as_ptr(), std::ptr::null(), 0) } {
			0 => true,
			_ => false,
		}
	}
}

struct Context {
	fs: Box<dyn Filesystem>,
}

pub unsafe fn get_filesystem() -> &'static mut dyn Filesystem {
	let ctx = &mut *fuse2::fuse_get_context();
	let data = &mut *(ctx.private_data as *mut Context);
	&mut *data.fs
}
fn map_path(path: *const c_char) -> &'static Path {
	Path::new(OsStr::from_bytes(unsafe { CStr::from_ptr(path) }.to_bytes()))
}

fn map_time(t: SystemTime) -> fuse2::timespec {
	let diff = t.duration_since(SystemTime::UNIX_EPOCH).unwrap();

	fuse2::timespec {
		tv_sec: diff.as_secs() as i64,
		tv_nsec: diff.subsec_nanos() as i64,
	}
}

unsafe extern "C" fn fs_getattr(
	path: *const c_char,
	st: *mut fuse2::stat,
) -> c_int {
	let path = map_path(path);
	let fs = get_filesystem();
	let st = &mut *st;

	match fs.getattr(path) {
		Ok(attr) => {

			let kind = match attr.kind {
				FileType::RegularFile => libc::S_IFREG,
				FileType::Directory => libc::S_IFDIR,
				FileType::Symlink => libc::S_IFLNK,
				FileType::Socket => libc::S_IFSOCK,
				FileType::NamedPipe => libc::S_IFIFO,
				FileType::CharDevice => libc::S_IFCHR,
				FileType::BlockDevice => libc::S_IFBLK,
			};
			
			st.st_ino = attr.ino;
			st.st_size = attr.size as i64;
			st.st_blocks = attr.blocks as i64;
			st.st_atim = map_time(attr.atime);
			st.st_mtim = map_time(attr.mtime);
			st.st_ctim = map_time(attr.ctime);
			st.__st_birthtim = map_time(attr.btime);
			st.st_mode = kind | attr.perm as u32;
			st.st_nlink = attr.nlink;
			st.st_uid = attr.uid;
			st.st_gid = attr.gid;
			st.st_rdev = attr.rdev as i32;
			st.st_blksize = attr.blksize as i32;
			st.st_flags = attr.flags;
			0
		},
		Err(e) => -e.raw_os_error().unwrap_or(libc::EIO),
	}
}

unsafe extern "C" fn fs_readdir(
	path: *const c_char,
	data: *mut c_void,
	filler: fuse2::fuse_fill_dir_t,
	off: fuse2::off_t,
	_ffi: *mut fuse2::fuse_file_info,
) -> c_int {
	let path = map_path(path);
	let fs = get_filesystem();

	let mut filler = DirFiller {
		func: filler,
		data,
	};

	match fs.readdir(path, off as u64, &mut filler) {
		Ok(()) => 0,
		Err(e) => -e.raw_os_error().unwrap_or(libc::EIO),
	}
}

unsafe extern "C" fn fs_read(
	path: *const c_char,
	buf: *mut c_char,
	size: usize,
	off: fuse2::off_t,
	_ffi: *mut fuse2::fuse_file_info,
) -> c_int {
	let path = map_path(path);
	let fs = get_filesystem();
	let buf = std::slice::from_raw_parts_mut(buf as *mut u8, size);

	match fs.read(path, off as u64, buf) {
		Ok(n) => n as c_int,
		Err(e) => -e.raw_os_error().unwrap_or(libc::EIO),
	}
}

unsafe extern "C" fn fs_open(
	path: *const c_char,
	_ffi: *mut fuse2::fuse_file_info,
) -> c_int {
	let path = map_path(path);
	let fs = get_filesystem();

	match fs.open(path) {
		Ok(()) => 0,
		Err(e) => -e.raw_os_error().unwrap_or(libc::EIO),
	}
}

// TODO: Filesystem::statvfs()
unsafe extern "C" fn fs_statfs(
	path: *const c_char,
	st: *mut fuse2::statvfs,
) -> c_int {
	let path = map_path(path);
	let st = &mut *st;
	let fs = get_filesystem();

	match fs.statfs(path) {
		Ok(s) => {
			st.f_bsize = s.bsize.into();
			st.f_frsize = s.frsize.into();
			st.f_blocks = s.blocks;
			st.f_bfree = s.bfree;
			st.f_bavail = s.bavail;
			st.f_files = s.files;
			st.f_ffree = s.ffree;
			st.f_favail = s.favail;
			0
		},
		Err(e) => -e.raw_os_error().unwrap_or(libc::EIO),
	}
}

static FSOPS: fuse2::fuse_operations = fuse2::fuse_operations {
	access: None,
	bmap: None,
	getattr: Some(fs_getattr),
	readlink: None,
	getdir: None,
	mknod: None,
	mkdir: None,
	unlink: None,
	rmdir: None,
	symlink: None,
	rename: None,
	link: None,
	chmod: None,
	chown: None,
	truncate: None,
	utime: None,
	open: Some(fs_open),
	read: Some(fs_read),
	write: None,
	statfs: Some(fs_statfs),
	flush: None,
	release: None,
	fsync: None,
	setxattr: None,
	getxattr: None,
	listxattr: None,
	removexattr: None,
	opendir: None,
	readdir: Some(fs_readdir),
	releasedir: None,
	fsyncdir: None,
	init: None,
	destroy: None,
	create: None,
	ftruncate: None,
	fgetattr: None,
	lock: None,
	utimens: None,
};

pub fn xmount(mp: &Path, fs: Box<dyn Filesystem>, opts: Vec<CString>) -> Result<()> {
	// TODO: this sucks, find something better
	let mut mp = mp.as_os_str().as_bytes().to_vec();
	mp.push(b'\0');
	let Ok(mp) = CString::from_vec_with_nul(mp) else {
		return Err(Error::from_raw_os_error(libc::EINVAL))
	};

	let ctx = Box::new(Context {
		fs,
	});

	let mut args = opts
		.into_iter()
		.chain(once(mp))
		.map(|s| s.into_raw())
		.chain(once(std::ptr::null_mut()))
		.collect::<Vec<_>>();

	let argc = args.len() as i32 - 1;
	let argv = args.as_mut_ptr();

	match unsafe { fuse2::fuse_main(argc, argv, &FSOPS, Box::into_raw(ctx) as *mut c_void) } {
		0 => Ok(()),
		_ => Err(Error::from_raw_os_error(libc::EIO)),
	}
}
