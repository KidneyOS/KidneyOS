# System calls - `SYS_CHDIR` (#11)

```rust
fn chdir(path: *const c_char) -> i32;
```

Sets the current working directory for the running process to `path`. Returns 0 on success.

### Errors

- `EIO` - an I/O error occurred while looking up `path`
- `ENOENT` - `path` does not exist
- `EFAULT` - path is an invalid pointer
- `ENOTDIR` - `path` is not a directory
- `ELOOP` - too many levels of symlinks encountered looking up `path`
