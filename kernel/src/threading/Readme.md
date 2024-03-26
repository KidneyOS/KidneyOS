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

A brand new thread has two different stack frames allocated in the following order (bottom of stack to top):

* `PrepareThreadContext`.
    This is a stack frame that performs some cleanup of arguments to facilitate the next stack frame.
* `SwitchThreadsContext`.
    This used within the `context_switch` function to store this threads state.
    Upon creation, this will delegate the thread to run the next stack frame.

The full stack will look like:

```
+-----------------------+ Bottom (High Address)
| &run_thread           | PrepareThreadContext
| --------------------- |
| eip = &prepare_thread | SwitchThreadsContext
| ebp = 0               |
| ebx = 0               |
| esi = 0               |
| edi = 0               |
+-----------------------+ Top (Low Address)
```

More details on the internals of these stack frames can be found within code comments.

### Notes on Argument Types

The actual function to perform context switch (`context_switch`) takes in pointers to TCB's as arguments.
We must have that the address of the TCB is the same address as the stack pointer contained within the TCB.
That is, `&TCB == &TCB.stack_pointer`.
Thus, the TCB struct must be a C struct.

The context switch function must retain the TCB pointers in order for the `prepare_thread` and `run_thread` function to properly schedule our threads.
However, internally, the context switch function must derefence these pointers to actually find the address of the stack.

### Notes on Scheduling

When calling a scheduler function, such as `scheduler_yield`, we must ensure that the scheduler knows of the correct running and waiting to run threads. There are two cases to consider.

#### Switching to a Previously Running Thread

When switching to a thread that was previously running, we are under the assumption that this thread was in the middle of running the `switch_threads` function.
Likewise, the thread we are switching from is left in the middle of the `switch_threads` function.
This makes scheduling easy.
The actual context switch will return the TCB of the thread we just came from.
As well, the new thread will be able to set itself as the running thread.

#### Switching to a New Thread

In the case that a new thread is being started, it will not begin running in the middle of the `switch_threads` function.
Instead, it will begin to run the `prepare_thread` and then `run_thread` functions.
This gives the `run_thread` function the responsibility to enque our previously running thread in the scheduler and update the currently running thread.

## Context Switching

The function `switch_threads` is the public method for switching to a given thread.
This will ensure that thread's statuses are updated correctly.
The thread specified to switch to must be in the ready state.

Typically, you do not interact with `switch_threads` directly.
Rather, you use one of the scheduling functions provided in the mod file for the scheduling crate.
