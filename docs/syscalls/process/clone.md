# Clone

Spawns another thread

### Synopsis

```rs
kidneyos_syscalls::clone;

clone(
    flags: u32,
    stack: *mut u8,
    parent_tid: *mut Tid,
    tls: u32,
    child_tid: *mut Tid,
) -> i32
```

```c
#include "kidneyos.h"

int32_t clone(uint32_t flags, uint8_t *stack, Tid *parent_tid, uint32_t tls, Tid *child_tid);
```

### Description

Clone spawns another thead (new TCB with the same page manager) in the current process.

Most parameters are ignored and reserved for future use. These are flags, parent_tid, tls, and child_tid.

The `stack` argument is the value for the new threads `esp`. Threads cannot share a stack, so it is the responsibility of the callee to allocate new space for the thread (using brk).
This argument cannot be passed as NULL (although this is supported in Linux). This still needs to be done.

### Return value

On success, this function returns 0. On failure this function will return -1 (for various reasons, see source code).

The stack parameter is not checked for faults, only when the thread starts executing will this error be detected via interrupt.
