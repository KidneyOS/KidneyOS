# Pipe

Create a cross-process "pipe" FD

### Synopsis

```rs
kidneyos_syscalls::pipe;

pipe(fds: *mut i32) -> i32
```

```c
#include "kidneyos.h"

int32_t pipe(int32_t *fds);
```

### Description

Creates two fds that make up a directional pipe that can communicate across processes.
Pipe must be called with a pointer to two consecutive fd handles. After a successful called to pipe, the fds array is initialized as such:

 - `fds[0]` holds the read end of the pipe, and can be read from using the `read` syscall.
 - `fds[1]` holds the write end of the pipe, and can be written to using the `write` syscall.

Bytes written to `fds[1]` can be read from `fds[0]`. The pipes are reference counted, and when all read ends are closed, you may receive `-EPIPE` on proceeding writes.

### Return value

On success, this function will return 0. On error, one of the following values are returned:

 - `-EFAULT`: The fds argument was invalid or NULL.
 - `-EMFILE`: Too many fds were established for this process (> 2^16).
