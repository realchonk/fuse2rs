#if defined(__FreeBSD__) || defined(__linux__)
# define _FILE_OFFSET_BITS 64
# define FUSE_USE_VERSION 26
#endif
#include <fuse.h>

