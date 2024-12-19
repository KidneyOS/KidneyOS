# Waitpid
Suspend process execution for a specified duraiton.

### Synopsis

```rs
use kidneyos_syscalls::waitpid;

waitpid(pid: Pid, stat: *mut i32, options: i32) -> Pid
```

```c
#include "kidneyos.h"

Pid waitpid(Pid pid, int32_t *stat, int32_t options);
```

### Description
Blocks until the process with the process ID pid exits and retrieves its exit code and status, which are stored in stat.
If the specified process has already terminated, the function returns immediately and signals the operating system to release any resources associated with that process.

Currently, no options are supported.

### Return value
On success, the ID of the terminated process is returned. On error, -1 is returned.