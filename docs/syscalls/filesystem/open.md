# System calls - `SYS_OPEN` (#5)

```rust
const O_CREATE: usize = 0x40;

fn open(path: *const c_char, flags: usize) -> i32;
```

Opens a file descriptor to the given `path` (null-terminated). `flags` can be 0 or `O_CREATE`. Currently all files are
opened as read/write — since we only support one user, opening files as read-only can be done at the
library level rather than the kernel level.

### Errors

- `EIO` - an I/O error occurred while opening the file
- `ENOENT` - some portion of `path` does not exist
- `EROFS` - `O_CREATE` was specified for a read-only file system
- `EFAULT` - path is an invalid pointer
- `EISDIR` - `O_CREATE` was specified and `path` is an existing directory
- `EINVAL` - an invalid flag was specified, or `path` contains illegal characters for paths in the file system
- `EMFILE` - too many open file descriptors
- `ENOSPC` - `O_CREATE` was specified and there is no space left on the device
- `ELOOP` - too many levels of symlinks encountered looking up `path`
