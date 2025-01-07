# Brk

Increases/decreases the size of the heap

### Synopsis

```rs
kidneyos_syscalls::brk;

brk(ptr: *const u8) -> isize
```

```c
#include "kidneyos.h"

intptr_t brk(const uint8_t *ptr);
```

### Description

Brk resizes the heap so that the **end** of the heap is `ptr` (not-exclusive, meaning dereferencing ptr would result in a fault, but ptr - 1 would be okay).

Sbrk for linux is usually implemented as a C stdlib convenience, so we only expose brk here.

### Return value

On success, this function returns 0. On failure this function will return -1.

This function can fail for a few reasons:

 - The heap_end argument is in kernel space.
 - The heap_end argument is before the mounted heap VMA (e.g. we would have a negative sized heap).
 - The heap_end argument would extend the heap into another VMA (e.g. something mounted with mmap, or the stack).
