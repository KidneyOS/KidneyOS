# Scheduler Yield

Yields the running thread to the processor.

### Synopsis

```rs
use kidneyos_syscalls::scheduler_yield;

scheduler_yield() -> i32
```

```c
#include "kidneyos.h"

int getpid();
```

### Description

Yields the current thread to the scheduler.

### Return value

These functions are always successful and returns 0.
