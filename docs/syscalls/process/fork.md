# Fork - WIP

Creates a child process.

### Synopsis

```rs
use kidneyos_syscalls::fork;

fork() -> Pid
```

```c
#include "kidneyos.h"

Pid fork();
```

### Description

Creates a new process by duplicating the calling process. The new "child" process is a duplicate of the parent with the following differences:

- The child process has its own process ID and thread ID. Its parent process ID is set to the parent's process ID.

- The child does not inherit any of the parent's locks.

### Return value

On success, the PID of the child process is returned to the caller (the parent process), and a value of 0 is returned to the child. On failure, -1 is returned to the parent, and no child process is created.
