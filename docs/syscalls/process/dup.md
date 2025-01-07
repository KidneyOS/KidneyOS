# Dup

Duplicates a file handle

### Synopsis

```rs
kidneyos_syscalls::dup;

dup(fd: i32) -> i32
```

```c
#include "kidneyos.h"

int32_t dup(int32_t fd);
```

### Description

Returns another file handle that acts the same as the handle argument `fd`.

This increments/clones any references to the original `fd`, so you are free to close the original fd knowing that a reference is maintained in the returned fd.

### Return value

On success, this function returns a valid FD. On error, one of the following values are returned:

 - `-EBADF`: The `fd` argument did not refer to a valid file descriptor for this process.
 - `-EMFILE`: Too many fds were established for this process (> 2^16).
