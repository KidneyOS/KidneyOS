# System calls - `SYS_GETDENTS` (#141)

```rust
const S_REGULAR_FILE: u8 = 1;
const S_SYMLINK: u8 = 2;
const S_DIRECTORY: u8 = 3;

struct Dirent {
    offset: i64,
    inode: u32,
    reclen: u16,
    r#type: u8,
    name: [u8; 0],
}

fn getdents(fd: usize, output: *mut Dirent, size: usize) -> isize;
```

Read directory entries from `fd` into `output`. `size` indicates the maximum number
of bytes worth of directory entries that may be read. Returns the number of bytes successfully read.
This may be less than `size`, or even zero if `size` is not large enough to hold a single directory entry.

The directory entries are placed adjacent to each other in memory, with `reclen` indicating the
offset in bytes from the start of one entry to the start of the next.
`r#type` is one of `S_REGULAR_FILE`, `S_SYMLINK`, or `S_DIRECTORY`,
to indicate the type of the entry. `offset` is an opaque value which can be passed to `lseek64`.
`inode` is the inode number of the directory entry. `name` is a variable-sized null-terminated string indicating name of the entry.

### Errors

- `EIO` - an I/O error occurred while reading from the directory
- `EBADF` - `fd` is invalid
- `EFAULT` - `output` is an invalid pointer
- `ENOTDIR` - `fd` is not a directory

