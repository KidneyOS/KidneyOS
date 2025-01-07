# Dup2

Changes the file descriptor table to "reconfigure" the value of an fd. "Clones" an fd into another fd value.

### Synopsis

```rs
kidneyos_syscalls::dup2;

fn dup2(old_fd: i32, new_fd: i32) -> i32
```

```c
#include "kidneyos.h"

int32_t dup2(int32_t old_fd, int32_t new_fd);
```

### Description

Reconfigures `old_fd` to refer to `new_fd` in the file descriptor table.
You can think of this as "redirecting" writes and read syscalls from `old_fd` to `new_fd`.

This increments/clones any references to `new_fd`, so you are free to close the original fd knowing that a reference is maintained.

### Return value

On success, this function returns a valid FD. On error, one of the following values are returned:

- `-EBADF`: The `old_fd` or `new_fd` arguments did not refer to a valid file descriptor for this process.
