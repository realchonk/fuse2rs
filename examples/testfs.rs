use std::{
	io::{Error, Result},
	path::Path,
};

use fuse2rs::*;

struct Testfs;

const TEXT: &[u8] = b"Hello World\n";

impl Filesystem for Testfs {
	fn getattr(&mut self, _req: &Request, path: &Path) -> Result<FileAttr> {
		if path == Path::new("/") {
			Ok(FileAttr {
				kind: FileType::Directory,
				perm: 0o755,
				size: 512,
				nlink: 2,
				..FileAttr::default()
			})
		} else if path == Path::new("/test") {
			Ok(FileAttr {
				kind: FileType::RegularFile,
				perm: 0o644,
				size: TEXT.len() as u64,
				..FileAttr::default()
			})
		} else {
			Err(Error::from_raw_os_error(libc::ENOENT))
		}
	}

	fn readdir(
		&mut self,
		_req: &Request,
		path: &Path,
		off: u64,
		filler: &mut DirFiller,
		_info: &FileInfo,
	) -> Result<()> {
		if path != Path::new("/") {
			return Err(Error::from_raw_os_error(libc::ENOENT));
		}

		if off != 0 {
			return Ok(());
		}

		filler.push(c".");
		filler.push(c"..");
		filler.push(c"test");

		Ok(())
	}

	fn read(
		&mut self,
		_req: &Request,
		path: &Path,
		off: u64,
		buf: &mut [u8],
		_info: &FileInfo,
	) -> Result<usize> {
		if path != Path::new("/test") {
			return Err(Error::from_raw_os_error(libc::ENOENT));
		}

		let off = off as usize;

		if off >= TEXT.len() {
			return Ok(0);
		}

		let len = if off + buf.len() <= TEXT.len() {
			buf.len()
		} else {
			TEXT.len() - off
		};

		buf[0..len].copy_from_slice(&TEXT[off..(off + len)]);

		Ok(len)
	}
}

fn main() {
	let mp = std::env::args_os().nth(1).unwrap();
	let args = vec![
		MountOption::Foreground,
		MountOption::Debug,
		MountOption::AllowOther,
	];
	fuse2rs::mount(Path::new(&mp), Testfs, args).unwrap();
}
