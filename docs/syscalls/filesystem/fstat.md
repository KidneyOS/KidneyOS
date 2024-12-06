# System calls - `SYS_FSTAT` (#108)

```rust
const S_REGULAR_FILE: u8 = 1;
const S_DIRECTORY: u8 = 3;

struct Stat {
    inode: u32,
    nlink: u32,
    size: u64,
    r#type: u8,
}

fn fstat(fd: i32, statbuf: *mut Stat) -> isize;
```

Retrieves file information from `fd`, placing it into `statbuf`. Returns 0 on success.
`inode` is the inode number of the file, `nlink` is the number of links to the file,
`size` is the size of the file in bytes, and `r#type` is either `S_REGULAR_FILE` or `S_DIRECTORY`,
to indicate a regular file or directory respectively.

### Errors

- `EIO` - an I/O error occurred while getting file information
- `EBADF` - `fd` is invalid or not stat-able
- `EFAULT` - `statbuf` is an invalid pointer
