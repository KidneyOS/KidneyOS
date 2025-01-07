# Nanosleep

### Synopsis

```rs
use kidneyos_syscalls::nanosleep;

nanosleep(duration: &Timespec, rem: &mut Timespec) -> i32;
```

```c
#include "kidneyos.h"

int nanosleep(struct timespec *duration, struct timespec *rem);
```

### Description

Suspends the execution of the calling thread for at least the duration specified in `duration`.

Currently there is no way to interrupt a nanosleep call except for exiting the process, and `rem` is unused.

### Return value

After successfully sleeping for `duration` 0 is returned. -1 is returned on error.
