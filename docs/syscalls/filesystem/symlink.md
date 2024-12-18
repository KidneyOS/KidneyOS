# System calls - `SYS_SYMLINK` (#83)

```rust
fn symlink(source: *const c_char, dest: *const c_char) -> isize;
```

Create `dest` to be a symbolic link to the path `source`. Returns 0 on success.

### Errors

- `EIO` - an I/O error occurred while creating the link
- `EINVAL` - `dest` contains illegal characters for paths in the file system
- `EFAULT` - `source` or `dest` is an invalid pointer
- `EEXIST` - `dest` already exists
- `ENOSPC` - not enough space on device to create symlink
- `EROFS` - `dest` lies in a read-only file system
- `ELOOP` - too many levels of symlinks encountered looking up `source` or `dest`
