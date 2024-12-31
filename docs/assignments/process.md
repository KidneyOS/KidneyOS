# Process implementation details.

### Process Table

Process control blocks (PCBs) are stored in a global process table.
PCBs are placed into the table when created (either on initialization or a call to fork) and remain there until a successful call to waitpid.

The process table is implemented internally using a binary tree mapping each process' ID to its PCB. Each PCB is kept within a mutex to ensure thread safety.

### Process creation and destruction

Processes are created via the `fork` syscall, duplicating the current process's execution context.

When a process is destroyed via a call to `exit`, the calling thread and all other threads of the process are destroyed, and their resources are freed.
The PCB associated with the process is kept in the process table until another process calls `wait` with the exited process' PID.
Any child process is inherited by the exited process' parent.

### Scheduler

A simple FIFO-based scheduler is implemented using Rust's VecDeque.

### Sleep Queue

The queue for blocked or sleeping threads is not represented as a separate queue.
Instead, each thread control block in the ready queue contains a status enum that can be set to ThreadStatus::Blocked.
When a blocked thread reaches the front of the ready queue, it is skipped and placed back at the end of the queue.

### Synchronization

Several locking primitives are implemented within KidneyOS to maintain multi-threaded synchronization.
Two mutex locks are implemented, both presenting and using a similar API to that of the Rust standard library, returning an RAII guard to access the protected data. One mutex uses a ticketing system with busy waiting, while the second, a sleeping mutex, leverages the thread sleep functionality described above.
