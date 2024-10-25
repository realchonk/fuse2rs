use std::{
	ffi::*,
	io::{Error, Result},
	iter::once,
	os::unix::ffi::OsStrExt,
	path::Path,
	time::{Duration, SystemTime},
};
use cfg_if::cfg_if;

use crate::{FileInfo, FileType, Filesystem, Request};

use self::fuse2::{dev_t, fuse_file_info, fuse_fill_dir_t, gid_t, mode_t, off_t, timespec, uid_t, utimbuf};

#[allow(
	dead_code,
	unused_variables,
	non_camel_case_types,
	non_snake_case,
	non_upper_case_globals
)]
mod fuse2 {
	include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub struct DirFiller {
	func: fuse2::fuse_fill_dir_t,
	data: *mut c_void,
}

impl DirFiller {
	pub fn push(&mut self, name: &CStr /* TODO: off, st */) -> bool {
		match unsafe { self.func.unwrap()(self.data, name.as_ptr(), std::ptr::null(), 0) } {
			0 => true,
			_ => false,
		}
	}
}

struct Context {
	fs: Box<dyn Filesystem>,
}

pub unsafe fn request() -> (&'static mut dyn Filesystem, Request) {
	let ctx = &mut *fuse2::fuse_get_context();
	let data = &mut *(ctx.private_data as *mut Context);
	let req = Request {
		uid:   ctx.uid,
		gid:   ctx.uid,
		umask: ctx.umask,
	};
	(&mut *data.fs, req)
}
fn map_path(path: *const c_char) -> &'static Path {
	Path::new(OsStr::from_bytes(
		unsafe { CStr::from_ptr(path) }.to_bytes(),
	))
}

fn map_time(t: SystemTime) -> fuse2::timespec {
	let diff = t.duration_since(SystemTime::UNIX_EPOCH).unwrap();

	fuse2::timespec {
		tv_sec:  diff.as_secs() as i64,
		tv_nsec: diff.subsec_nanos() as i64,
	}
}

fn map_err(e: Error) -> i32 {
	-e.raw_os_error().unwrap_or(libc::EIO)
}
fn map(r: Result<()>) -> i32 {
	match r {
		Ok(()) => 0,
		Err(e) => map_err(e),
	}
}

impl From<&fuse2::fuse_file_info> for FileInfo {
	fn from(info: &fuse2::fuse_file_info) -> Self {
		Self {
			fh:          info.fh,
			flags:       info.flags,
			flush:       info.flush() != 0,
			direct_io:   info.direct_io() != 0,
			keep_cache:  info.keep_cache() != 0,
			nonseekable: info.nonseekable() != 0,
		}
	}
}

impl FileInfo {
	fn write(&self, info: &mut fuse2::fuse_file_info) {
		info.fh = self.fh;
		info.set_flush(self.flush as u32);
		info.set_direct_io(self.direct_io as u32);
		info.set_keep_cache(self.keep_cache as u32);
		info.set_nonseekable(self.nonseekable as u32);
	}
}

unsafe extern "C" fn fs_getattr(path: *const c_char, st: *mut fuse2::stat) -> c_int {
	let path = map_path(path);
	let (fs, req) = request();
	let st = &mut *st;

	map(
		fs
			.getattr(&req, path)
			.map(|attr| {
				let kind = match attr.kind {
					FileType::RegularFile => libc::S_IFREG,
					FileType::Directory => libc::S_IFDIR,
					FileType::Symlink => libc::S_IFLNK,
					FileType::Socket => libc::S_IFSOCK,
					FileType::NamedPipe => libc::S_IFIFO,
					FileType::CharDevice => libc::S_IFCHR,
					FileType::BlockDevice => libc::S_IFBLK,
				} as u32;

				st.st_ino = attr.ino;
				st.st_size = attr.size as i64;
				st.st_blocks = attr.blocks as i64;
				st.st_atim = map_time(attr.atime);
				st.st_mtim = map_time(attr.mtime);
				st.st_ctim = map_time(attr.ctime);
				cfg_if! {
					if #[cfg(target_os = "openbsd")] {
						st.__st_birthtim = map_time(attr.btime);
					} else if #[cfg(target_os = "freebsd")] {
						st.st_birthtim = map_time(attr.btime);
					} else {
					}
				}
				st.st_mode = (kind | attr.perm as u32).try_into().unwrap();
				st.st_nlink = attr.nlink.try_into().unwrap();
				st.st_uid = attr.uid;
				st.st_gid = attr.gid;
				st.st_rdev = attr.rdev.try_into().unwrap();
				st.st_blksize = attr.blksize.try_into().unwrap();
				cfg_if! {
					if #[cfg(any(target_os = "openbsd", target_os = "freebsd"))] {
						st.st_flags = attr.flags;
					}
				}
			})
	)
}

unsafe extern "C" fn fs_readdir(
	path: *const c_char,
	data: *mut c_void,
	filler: fuse_fill_dir_t,
	off: off_t,
	ffi: *mut fuse_file_info,
) -> c_int {
	let path = map_path(path);
	let (fs, req) = request();

	let mut filler = DirFiller { func: filler, data };

	let info = FileInfo::from(&*ffi);
	map(fs.readdir(&req, path, off as u64, &mut filler, &info))
}

unsafe extern "C" fn fs_read(
	path: *const c_char,
	buf: *mut c_char,
	size: usize,
	off: off_t,
	ffi: *mut fuse_file_info,
) -> c_int {
	let path = map_path(path);
	let (fs, req) = request();
	let info = FileInfo::from(&*ffi);
	let buf = std::slice::from_raw_parts_mut(buf as *mut u8, size);

	match fs.read(&req, path, off as u64, buf, &info) {
		Ok(n) => n as c_int,
		Err(e) => map_err(e),
	}
}

unsafe extern "C" fn fs_write(
	path: *const c_char,
	buf: *const c_char,
	size: usize,
	off: off_t,
	ffi: *mut fuse_file_info,
) -> c_int {
	let path = map_path(path);
	let (fs, req) = request();
	let info = FileInfo::from(&*ffi);
	let buf = std::slice::from_raw_parts(buf as *const u8, size);

	match fs.write(&req, path, off as u64, buf, &info) {
		Ok(n) => n as c_int,
		Err(e) => map_err(e),
	}
}

unsafe extern "C" fn fs_open(path: *const c_char, ffi: *mut fuse_file_info) -> c_int {
	let path = map_path(path);
	let (fs, req) = request();
	let mut info = FileInfo::from(&*ffi);

	map(
		fs
			.open(&req, path, &mut info)
			.map(|_| info.write(&mut *ffi))
	)
}

unsafe extern "C" fn fs_opendir(path: *const c_char, ffi: *mut fuse_file_info) -> c_int {
	let path = map_path(path);
	let (fs, req) = request();
	let mut info = FileInfo::from(&*ffi);

	map(
		fs
			.opendir(&req, path, &mut info)
			.map(|_| info.write(&mut *ffi))
	)
}

unsafe extern "C" fn fs_statfs(path: *const c_char, st: *mut fuse2::statvfs) -> c_int {
	let path = map_path(path);
	let st = &mut *st;
	let (fs, req) = request();

	map(
		fs
			.statfs(&req, path)
			.map(|s| {
				st.f_bsize = s.bsize.into();
				st.f_frsize = s.frsize.into();
				st.f_blocks = s.blocks;
				st.f_bfree = s.bfree;
				st.f_bavail = s.bavail;
				st.f_files = s.files;
				st.f_ffree = s.ffree;
				st.f_favail = s.favail;
			})
	)
}

unsafe extern "C" fn fs_init(_info: *mut fuse2::fuse_conn_info) -> *mut c_void {
	let ctx = &mut *fuse2::fuse_get_context();
	let data = &mut *(ctx.private_data as *mut Context);
	let req = Request {
		uid:   ctx.uid,
		gid:   ctx.uid,
		umask: ctx.umask,
	};
	data.fs.init(&req);
	ctx.private_data
}

unsafe extern "C" fn fs_destroy(_ptr: *mut c_void) {
	let (fs, _req) = request();
	fs.destroy();
}

unsafe extern "C" fn fs_readlink(path: *const c_char, buf: *mut c_char, size: usize) -> c_int {
	let path = map_path(path);
	let buf = std::slice::from_raw_parts_mut(buf as *mut u8, size);
	let (fs, req) = request();

	map(fs.readlink(&req, path, buf))
}

unsafe extern "C" fn fs_release(path: *const c_char, ffi: *mut fuse_file_info) -> c_int {
	let path = map_path(path);
	let info = FileInfo::from(&*ffi);
	let (fs, req) = request();

	map(fs.release(&req, path, &info))
}

unsafe extern "C" fn fs_flush(path: *const c_char, ffi: *mut fuse_file_info) -> c_int {
	let path = map_path(path);
	let info = FileInfo::from(&*ffi);
	let (fs, req) = request();

	map(fs.flush(&req, path, &info))
}

unsafe extern "C" fn fs_releasedir(path: *const c_char, ffi: *mut fuse_file_info) -> c_int {
	let path = map_path(path);
	let info = FileInfo::from(&*ffi);
	let (fs, req) = request();

	map(fs.releasedir(&req, path, &info))
}

unsafe extern "C" fn fs_unlink(path: *const c_char) -> c_int {
	let path = map_path(path);
	let (fs, req) = request();

	map(fs.unlink(&req, path))
}

unsafe extern "C" fn fs_rmdir(path: *const c_char) -> c_int {
	let path = map_path(path);
	let (fs, req) = request();

	map(fs.rmdir(&req, path))
}

unsafe extern "C" fn fs_mkdir(path: *const c_char, mode: mode_t) -> c_int {
	let path = map_path(path);
	let (fs, req) = request();

	map(fs.mkdir(&req, path, mode as u32))
}

unsafe extern "C" fn fs_mknod(path: *const c_char, mode: mode_t, dev: dev_t) -> c_int {
	let path = map_path(path);
	let (fs, req) = request();

	map(fs.mknod(&req, path, mode as u32, dev as u32))
}

unsafe extern "C" fn fs_create(path: *const c_char, mode: mode_t, ffi: *mut fuse_file_info) -> c_int {
	let path = map_path(path);
	let (fs, req) = request();
	let info = FileInfo::from(&*ffi);

	map(fs.create(&req, path, mode as u32, &info))
}

unsafe extern "C" fn fs_chown(path: *const c_char, uid: uid_t, gid: gid_t) -> c_int {
	let path = map_path(path);
	let uid = if uid < u32::MAX { Some(uid) } else { None };
	let gid = if gid < u32::MAX { Some(gid) } else { None };
	let (fs, req) = request();

	map(fs.chown(&req, path, uid, gid))
}

unsafe extern "C" fn fs_chmod(path: *const c_char, mode: mode_t) -> c_int {
	let path = map_path(path);
	let mode = mode as u32;
	let (fs, req) = request();

	map(fs.chmod(&req, path, mode))
}

unsafe extern "C" fn fs_utime(path: *const c_char, buf: *mut utimbuf) -> c_int {
	let path = map_path(path);
	let (fs, req) = request();

	let (at, mt) = if buf.is_null() {
		let now = SystemTime::now();
		(now, now)
	} else {
		let buf = &*buf;
		let f = |t: i64| {
			if t >= 0 {
				SystemTime::UNIX_EPOCH + Duration::new(t as u64, 0)
			} else {
				SystemTime::UNIX_EPOCH - Duration::new(-t as u64, 0)
			}
		};
		(f(buf.actime), f(buf.modtime))
	};

	map(fs.utime(&req, path, at, mt))
}

unsafe extern "C" fn fs_utimens(path: *const c_char, ts: *const timespec) -> c_int {
	let path = map_path(path);
	let (fs, req) = request();

	let (at, mt) = if ts.is_null() {
		let now = SystemTime::now();
		(now, now)
	} else {
		let f = |t: timespec| {
			if t.tv_sec >= 0 {
				SystemTime::UNIX_EPOCH + Duration::new(t.tv_sec as u64, t.tv_nsec as u32)
			} else {
				SystemTime::UNIX_EPOCH - Duration::new(-t.tv_sec as u64, t.tv_nsec as u32)
			}
		};
		(f(ts.read()), f(ts.add(1).read()))
	};

	map(fs.utime(&req, path, at, mt))
}

unsafe extern "C" fn fs_link(name1: *const c_char, name2: *const c_char) -> c_int {
	let name1 = map_path(name1);
	let name2 = map_path(name2);
	let (fs, req) = request();

	map(fs.link(&req, name1, name2))
}

unsafe extern "C" fn fs_symlink(name1: *const c_char, name2: *const c_char) -> c_int {
	let name1 = map_path(name1);
	let name2 = map_path(name2);
	let (fs, req) = request();

	map(fs.symlink(&req, name1, name2))
}

unsafe extern "C" fn fs_rename(from: *const c_char, to: *const c_char) -> c_int {
	let from = map_path(from);
	let to = map_path(to);
	let (fs, req) = request();

	map(fs.rename(&req, from, to))
}

unsafe extern "C" fn fs_truncate(path: *const c_char, size: off_t) -> c_int {
	let path = map_path(path);
	let (fs, req) = request();

	map(fs.truncate(&req, path, size as u64))
}


static FSOPS: fuse2::fuse_operations = fuse2::fuse_operations {
	access: None,
	bmap: None,
	getattr: Some(fs_getattr),
	readlink: Some(fs_readlink),
	getdir: None,
	mknod: Some(fs_mknod),
	mkdir: Some(fs_mkdir),
	unlink: Some(fs_unlink),
	rmdir: Some(fs_rmdir),
	symlink: Some(fs_symlink),
	rename: Some(fs_rename),
	link: Some(fs_link),
	chmod: Some(fs_chmod),
	chown: Some(fs_chown),
	truncate: Some(fs_truncate),
	utime: Some(fs_utime),
	open: Some(fs_open),
	read: Some(fs_read),
	write: Some(fs_write),
	statfs: Some(fs_statfs),
	flush: Some(fs_flush),
	release: Some(fs_release),
	fsync: None,
	setxattr: None,
	getxattr: None,
	listxattr: None,
	removexattr: None,
	opendir: Some(fs_opendir),
	readdir: Some(fs_readdir),
	releasedir: Some(fs_releasedir),
	fsyncdir: None,
	init: Some(fs_init),
	destroy: Some(fs_destroy),
	create: Some(fs_create),
	ftruncate: None,
	fgetattr: None,
	lock: None,
	utimens: Some(fs_utimens),

	// this is _very_ ugly
	..unsafe { std::mem::zeroed() }
};

pub fn xmount(mp: &Path, fs: Box<dyn Filesystem>, opts: Vec<CString>) -> Result<()> {
	// TODO: this sucks, find something better
	let mut mp = mp.as_os_str().as_bytes().to_vec();
	mp.push(b'\0');
	let Ok(mp) = CString::from_vec_with_nul(mp) else {
		return Err(Error::from_raw_os_error(libc::EINVAL));
	};

	let ctx = Box::new(Context { fs });

	let mut args = opts
		.into_iter()
		.chain(once(mp))
		.map(|s| s.into_raw())
		.chain(once(std::ptr::null_mut()))
		.collect::<Vec<_>>();

	let argc = args.len() as i32 - 1;
	let argv = args.as_mut_ptr();
	let ctx = Box::into_raw(ctx) as *mut c_void;

	let ec;
	cfg_if! {
		if #[cfg(any(target_os = "freebsd", target_os = "linux"))] {
			ec = unsafe {
				fuse2::fuse_main_real(argc, argv, &FSOPS, std::mem::size_of_val(&FSOPS), ctx)
			};
		} else {
			ec = unsafe {
				fuse2::fuse_main(argc, argv, &FSOPS, ctx)
			};
		}
	};
	match ec {
		0 => Ok(()),
		_ => Err(Error::from_raw_os_error(libc::EIO)),
	}
}
