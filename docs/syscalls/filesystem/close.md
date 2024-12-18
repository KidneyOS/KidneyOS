# System calls - `SYS_CLOSE` (6)

```rust
fn close(fd: i32) -> i32;
```

Closes the file descriptor `fd`. Returns 0 on success.

### Errors

- `EBADF` - `fd` is invalid
