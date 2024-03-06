# KidneyOS: Threading

Here is some documentation relating to threads within KidneyOS.

## Thread Creation

A new thread can be created via the `ThreadControlBlock::create` method.
This will go through the process of allocating a stack for the thread and preparing the initial stack frames.
A thread created from this function will be ready to be handed to the scheduler or run via the `switch_threads` method.

Stacks are allocated as a predetermined size (4KB).

The stack frames required to get a thread running are opaque to the user.
For developers, the details are here:

### Running a New Thread

A brand new thread has three different stack frames allocated in the following order (bottom of stack to top):

* `RunThreadContext`.
    This stack frame will actually run the function that the thread is specified to run.
    It also puts us in a safe place by enabling interrupts for the thread and safely exiting once a thread ends execution.
* `PrepareThreadContext`.
    This is a stack frame that performs some cleanup of arguments to facilitate the next stack frame.
* `SwitchThreadsContext`.
    This used within the `context_switch` function to store this threads state.
    Upon creation, this will delegate the thread to run the next stack frame.

The full stack will look like:

```
+-----------------------+ Bottom (High Address)
| eip = 0               | RunThreadContext
| &entry_function       |
| --------------------- |
| eip = &run_thread     | PrepareThreadContext
| --------------------- |
| eip = &prepare_thread | SwitchThreadsContext
| ebp = 0               |
| ebx = 0               |
| esi = 0               |
| edi = 0               |
+-----------------------+ Top (Low Address)
```

More details on the internals of these stack frames can be found within code comments.
