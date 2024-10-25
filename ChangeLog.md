# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),

## [0.1.2] - 2024-10-26

### Removed

- MountOption::Rw (was broken before)

## [0.1.1] - 2024-10-25

### Fixed

- Filesystem::flush()

## [0.1.0] - 2024-10-25

### Added

- Filesystem::unlink()
- Filesystem::rmdir()
- Filesystem::mkdir()
- Filesystem::mknod()
- Filesystem::chown()
- Filesystem::chmod()
- Filesystem::utime()
- Filesystem::write()
- Filesystem::link()
- Filesystem::symlink()
- Filesystem::rename()
- Filesystem::truncate()
- Filesystem::create()
- Filesystem::flush()

## [0.0.3] - 2024-10-25

### Added

- support for FreeBSD & Linux

### Fixed

- `fs_init()`

## [0.0.2] - 2024-09-22

### Fixed

- Filesystem::readlink()

## [0.0.1] - 2024-09-22

This was the first release of fuse2rs.

[0.1.2]: https://github.com/realchonk/fuse2rs/compare/0.1.1...0.1.2
[0.1.1]: https://github.com/realchonk/fuse2rs/compare/0.1.0...0.1.1
[0.1.0]: https://github.com/realchonk/fuse2rs/compare/0.0.3...0.1.0
[0.0.3]: https://github.com/realchonk/fuse2rs/compare/0.0.2...0.0.3
[0.0.2]: https://github.com/realchonk/fuse2rs/compare/0.0.1...0.0.2
[0.0.1]: https://github.com/realchonk/fuse2rs/releases/tag/0.0.1
