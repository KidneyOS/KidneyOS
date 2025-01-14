# System calls - `SYS_UNLINK` (#10)

```rust
fn unlink(path: *const c_char) -> isize;
```

Deletes the file `path`. If there are no remaining hard links, file descriptors, or memory mappings to the
underlying inode, the disk space taken up by the file is freed. Returns 0 on success.

### Errors

- `ENOENT` - `path` does not exist
- `EIO` - an I/O error occurred while unlinking the file
- `EFAULT` - `path` is an invalid pointer
- `EISDIR` - `path` points to a directory
- `EROFS` - `path` is inside a read-only file system
- `ELOOP` - too many levels of symlinks encountered looking up `path`
