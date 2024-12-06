# System calls - `SYS_LINK` (#9)

```rust
fn link(source: *const c_char, dest: *const c_char) -> isize;
```

Create `dest` to be a hard link to the file `source`. Returns 0 on success.

### Errors

- `EIO` - an I/O error occurred while creating the link
- `EINVAL` - `dest` contains illegal characters for paths in the file system
- `EFAULT` - `source` or `dest` is an invalid pointer
- `EEXIST` - `dest` already exists
- `EXDEV` - `source` and `dest` lie in different file systems
- `EISDIR` - `source` is a directory (as on Linux, directory hard links are forbidden)
- `ENOSPC` - not enough space on device to create hard link
- `EROFS` - `dest` lies in a read-only file system
- `EMLINK` - the number of hard links to `source` is already at the maximum number supported by the file system
- `ELOOP` - too many levels of symlinks encountered looking up `source` or `dest`
