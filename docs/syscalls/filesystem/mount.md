# System calls - `SYS_MOUNT` (#21)

```rust
fn mount(device: *const c_char, target: *const c_char, fs_type: *const c_char) -> isize;
```

Mounts the given device to the target directory, using the given file system type. Returns 0 on success.
Currently the following file system types are supported:

- `"tmpfs"` - in-memory file system. `device` should be set to `""`.

### Errors

- `EIO` - an I/O error occurred while mounting the device
- `EINVAL` - `target` contains illegal characters for paths in the file system
- `ENOENT` - `target` or `device` does not exist
- `EFAULT` - `target`, `device`, or `fs_type` is an invalid pointer
- `ENOTDIR` - `target` is not a directory
- `ENOTEMPTY` - `target` is not empty
- `ENODEV` - `fs_type` is not a supported file system
- `ELOOP` - too many levels of symlinks encountered looking up `device` or `target`
