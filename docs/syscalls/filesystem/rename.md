# System calls - `SYS_RENAME` (#38)

```rust
fn rename(source: *const c_char, dest: *const c_char) -> isize;
```

Renames `source` to `dest`. Returns 0 on success.

### Errors

- `EIO` - an I/O error occurred while renaming
- `EINVAL` - `dest` contains illegal characters for paths in the file system
- `EFAULT` - `source` or `dest` is an invalid pointer
- `EEXIST` - `dest` already exists
- `EXDEV` - `source` and `dest` lie in different file systems (currently cross-file-system renaming
            can only be done manually)
- `ENOSPC` - not enough space on device to perform rename
- `EROFS` - `source` or `dest` lies in a read-only file system
- `EMLINK` - the number of hard links to `source` is already at the maximum number supported by the file system
             (this may be an error due to implementing `rename` as `link(source, dest)` followed by `unlink(source)`)
- `ELOOP` - too many levels of symlinks encountered looking up `source` or `dest`
