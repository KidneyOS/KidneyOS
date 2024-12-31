# Execve

Execute a program

### Synopsis

```rs
kidneyos_syscalls::execve;

execve(filename: *const c_char, argv: *const *const c_char, envp: *const *const c_char) -> i32
```

```c
#include "kidneyos.h"

int32_t execve(const char *filename, const char *const *argv, const char *const *envp);
```

### Description

TODO

### Return value

On sucess, this function does not return, on error, -1 is returned.
