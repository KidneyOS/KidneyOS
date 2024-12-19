# Clock Gettime

Get the current system time.

### Synopsis

```rs
use kidneyos_syscalls::clock_gettime;

clock_gettime(clk_id: i32, ts: &mut Timespec) -> i32;
```

```c
#include "kidneyos.h"

int clock_gettime(int clk_id, struct timespec *rs);
```

### Description
Retrieves and sets the time according to the clock specified by `clk_id`.

Currently the following clocks are supported:

**CLOCK_REALTIME**
System-wide realtime clock.

**CLOCK_MONOTONIC**
Gets the monotonic time from some starting point. Currently represented by `rdtsc`

### Return value

Returns 0 on success and -1 on failure.