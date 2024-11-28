# System Calls

This assignment is still a work in progress. This page will be updated as it is worked on.

## Exit
Terminates the calling process.

#### Synopsis

```rs
kidneyos_syscalls::exit(code: i32) -> !
```

```c
#include "kidneyos.h"
void exit(int32_t status);
```

#### Description
The function `exit()` terminates the calling process. Any children of this process are inherited by the kernel process (process 1).
The value status is returned to the parent process as the process's exit status, and can be collected the `waitpid` system call.

#### Return value
This function does not return

## Fork - WIP
Creates a child process.

#### Synopsis

```rs
kidneyos_syscalls::fork() -> Pid
```

```c
#include "kidneyos.h"
Pid fork();
```

#### Description
Creates a new process by duplicating the calling process. The new "child" process is a duplicate of the parent with the following differences:

- The child process has its own process ID and thread ID. Its parent process ID is set to the parent's process ID.

- The child does not inherit any of the parent's locks.

#### Return value
On success, the PID of the child process is returned to the caller (the parent process), and a value of 0 is returned to the child. On failure, -1 is returned to the parent, and no child process is created.


## Waitpid
Wait for a process to terminate

#### Synopsis

```rs
kidneyos_syscalls::waitpid(pid: Pid, stat: *mut i32, options: i32) -> Pid
```

```c
#include "kidneyos.h"
Pid waitpid(Pid pid, int32_t *stat, int32_t options);
```

#### Description
Blocks until the process with the process ID pid exits and retrieves its exit code and status, which are stored in stat.
If the specified process has already terminated, the function returns immediately and signals the operating system to release any resources associated with that process.

Currently, no options are supported.

#### Return value
On success, the ID of the terminated process is returned. On error, -1 is returned.


## Execve
Execute a program

#### Synopsis

```rs
kidneyos_syscalls::execve(filename: *const c_char, argv: *const *const c_char, envp: *const *const c_char) -> i32
```

```c
#include "kidneyos.h"
int32_t execve(const char *filename, const char *const *argv, const char *const *envp);
```

#### Description
TODO

#### Return value
On sucess, this function does not return, on error, -1 is returned.

## Nanosleep

#### Synopsis

```rs

```

```c
#include "kidneyos.h"

```

#### Description
TODO

#### Return value