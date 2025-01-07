# Getpid

Get the running process' PID.

### Synopsis

```rs
use kidneyos_syscalls::getpid;

getpid() -> Pid
```

```c
#include "kidneyos.h"

Pid getpid();
```

### Description

Returns the process ID (Pid) of the of the calling process.

### Return value

These functions are always successful.
