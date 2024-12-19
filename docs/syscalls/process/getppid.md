# Getppid
Get the running process' parent PID.

### Synopsis

```rs
use kidneyos_syscalls::getppid;

getppid() -> Pid
```

```c
#include "kidneyos.h"

Pid getppid();
```

### Description
Returns the process ID (Pid) of the of the caller's parent process. 
This is either the Pid of the process that created this one using **fork()**, or the process that inherited this one after the parent's call to **exit**.

### Return value
These functions are always successful.