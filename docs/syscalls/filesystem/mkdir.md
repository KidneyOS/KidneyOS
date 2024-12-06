# System calls - `SYS_MKDIR` (#39)

```rust
fn mkdir(path: *const c_char) -> i32;
```

Creates a directory at `path` (null-terminated). Returns 0 on success.

### Errors

- `EIO` - an I/O error occurred while creating the directory
- `ENOENT` - some portion of `path` does not exist
- `EROFS` - `path` lies in a read-only file system
- `EFAULT` - `path` is an invalid pointer
- `EEXIST` - `path` already exists
- `EINVAL` - `path` contains illegal characters for paths in the file system
- `ENOSPC` - there is not enough space left on the device to create the directory
- `ELOOP` - too many levels of symlinks encountered looking up `path`
