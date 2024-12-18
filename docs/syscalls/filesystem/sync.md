# System calls - `SYS_SYNC` (#36)

```rust
fn sync() -> isize;
```

Syncs all file systems, blocking until any pending writes are committed to disk.
Returns 0 on success.

### Errors

- `EIO` - an I/O error occurred while syncing a device
