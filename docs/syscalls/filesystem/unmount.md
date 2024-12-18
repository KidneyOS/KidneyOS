# System calls - `SYS_UNMOUNT` (#22)

```rust
fn unmount(path: *const c_char) -> isize;
```

Unmounts the device mounted to `path`. Blocks until all pending writes to the file system
have completed (like `sync` does). Returns 0 on success.

### Errors

- `EIO` - an I/O error occurred while syncing the device 
- `EBUSY` - There are open file descriptors or memory mappings to files in the file system.
- `ENOENT` - `path` does not exist, or is not a mount point
- `EFAULT` - `path` is an invalid pointer
- `ELOOP` - too many levels of symlinks encountered looking up `path`
