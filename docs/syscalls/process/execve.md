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

Replaces the current process with another executable, specified by the filename argument.
The filename argument refers to an ELF-encoded x86 executable that will be parsed and decoded.

This is accomplished by dropping the old TCB and creating a new one with the same PID (process parent).
Multiple threads per processes is not extensively supported here, so extra handling needs to be done to properly handle this in a process with many threads.

envp is currently ignored, as there is no way for the guest executable to access arguments.

### Return value

On success, this function does not return. On error, one of the following values are returned:

 - `-EFAULT`: The filename, argv, or envp arguments were invalid (do not point to a valid page, or point to NULL).
 - `-ENOENT`: The filename argument was not a valid UTF-8 encoded string.
 - `-EIO`: Filename did not refer to a valid file, or there was a problem reading the file at this path.
 - `-ENOEXEC`: The file specified by filename was not an ELF file, or not an executable ELF, or the ELF architecture is not set to x86.
