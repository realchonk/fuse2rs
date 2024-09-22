use std::{
	fmt::{self, Display, Formatter}, process::{Child, Command}, thread::sleep, time::{Duration, Instant}
};
use tempfile::{tempdir, TempDir};
use cfg_if::cfg_if;

struct Harness {
	dir: TempDir,
	child: Child,
}

#[derive(Debug, Clone, Copy)]
struct WaitForError;

impl Display for WaitForError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "timeout waiting for condition")
	}
}

impl std::error::Error for WaitForError {}

fn waitfor<C>(timeout: Duration, condition: C) -> Result<(), WaitForError>
where
	C: Fn() -> bool,
{
	let start = Instant::now();
	loop {
		if condition() {
			break Ok(());
		}
		if start.elapsed() > timeout {
			break (Err(WaitForError));
		}
		sleep(Duration::from_millis(50));
	}
}
impl Harness {
	fn new() -> Self {
		let dir = tempdir().unwrap();

		let child = Command::new("doas")
			.arg("target/debug/examples/testfs")
			.arg(dir.path())
			.spawn()
			.unwrap();
		
		waitfor(Duration::from_secs(5), || {
			let s = nix::sys::statfs::statfs(dir.path()).unwrap();

			cfg_if! {
				if #[cfg(target_os = "openbsd")] {
					s.filesystem_type_name() == "fuse"
				}
			}
			
		}).unwrap();
		
		Self {
			dir,
			child,
		}
	}
}

#[test]
fn testfs() {
	let h = Harness::new();
	drop(h);
}
