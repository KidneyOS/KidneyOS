# Exit
Terminates the calling process.

### Synopsis

```rs
use kidneyos_syscalls::exit;

exit(code: i32) -> !
```

```c
#include "kidneyos.h"

void exit(int32_t code);
```

### Description
The function `exit()` terminates the calling process. Any children of this process are inherited by the init process (process ID 1).
The value of `code` is returned to the parent process as the process's exit status, and can be collected the `waitpid` system call.

### Return value
This function does not return
