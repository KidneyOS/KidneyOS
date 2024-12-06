# System calls - `SYS_RMDIR` (#40)

```rust
fn rmdir(path: *const c_char) -> isize;
```

Deletes the directory at `path`, which must be empty. Returns 0 on success.

### Errors

- `ENOENT` - `path` does not exist
- `EIO` - an I/O error occurred while removing the directory
- `EFAULT` - `path` is an invalid pointer
- `ENOTDIR` - `path` is not a directory
- `ENOTEMPTY` - the directory at `path` is not empty
- `EROFS` - `path` is inside a read-only file system
- `ELOOP` - too many levels of symlinks encountered looking up `path`
