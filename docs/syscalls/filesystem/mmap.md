# System calls - `SYS_MMAP` (#90)

```rust
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const PROT_EXEC: i32 = 4;

struct MMapOptions {
    addr: *mut c_void,
    length: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    offset: i64,
}

fn mmap(options: *const MMapOptions) -> isize;
```

Create a memory mapping to the file referenced by `fd`, at the address `addr`.
`prot` indicates the memory protections, and should be a bitwise OR of `PROT_READ`, `PROT_WRITE`, and/or `PROT_EXEC`.
`flags` must be zero. `offset` indicates the offset into the file at which the mapping starts; this must be a multiple of the page size.
On success, returns the address at which the memory was mapped.

### Errors

- `EIO` - an I/O error occurred while mapping the file
- `EFAULT` - `options` is an invalid pointer
- `EISDIR` - `fd` is a directory
- `EINVAL` - `prot` or `flags` is invalid, or `length` is unreasonably long
- `EBADF` - `fd` is invalid, or not mappable
- `ENOMEM` - not enough free address space to create the mapping
- `ESPIPE` - `offset` is negative, or unreasonably large
