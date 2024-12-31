# Getrandom

Generates random bytes.

### Synopsis

```rs
use kidneyos_syscalls::getrandom;

getrandom(buf: *mut i8, size: usize, flags: usize) -> i32
```

```c
#include "kidneyos.h"

int getrandom(char *buf, size_t size, size_t flags);
```

### Description

Fills the buffer `buf` with `size` cryptographically random bytes. The random data is generated using the `rdrand` instruction.

**Flags:** Currently, no flags are supported. This parameter is reserved for future use and is unused.

### Return value

On success, the number of bytes generated is returned. On failure -1 is returned.
