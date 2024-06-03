# KidneyOS: Threading

Here is some documentation relating to threads within KidneyOS.

## Thread Control Blocks

Within KidneyOS, a thread is a unit of computation.
That is, it represents the execution of code within a program.
Each thread is tracked within the OS as a single Thread Control Block (TCB).
TCB's are where the information for a thread's stack, priority, and CPU registers are stored.

A TCB for a thread must remain in memory for the entire duration of the threads runtime.

### Ownership of Threads

Each thread, more specifically it's TCB, will have exactly one owner at any given time.
The kernel itself can own a single thread at any given time.
This is the TCB stored within `RUNNING_THREAD`.
This is also the thread that should be currently executing.
Notably, while a thread is running, it's TCB does not accurately reflect it's state; the state is only updated on context switches.

Every non-running thread must be owned by the scheduler (`SCHEDULER`).
The scheduler is responsible for determining the order of thread's to be run and provides a simple interface for the kernel to interact with.
These ownership rules allow for a large degree of freedom for the implementation of the scheduler (which is desirable since that is a student facing assignment).
The scheduler, when asked, will relinquish ownership of a thread to the kernel and may be given new threads from the kernel.

### Thread Creation

A new thread can be created via the [`ThreadControlBlock::new`](./thread_control_block.rs) method.
This will go through the process of allocating a stack for the thread and preparing the initial stack frames.
A thread created from this function will be ready to be handed to the scheduler or run via the `switch_threads` method.

Stacks are allocated as a predetermined size (1MB).

The stack frames required to get a thread running are opaque to the user.
For developers, the details are below.

### Thread Reaping

When a thread ends or is killed, it must be moved into the `Dying` state.
From here, the kernel can recognize a dying threads and reap the resources owned by the thread.
This includes freeing it's stack, heap, and removing it's access to any locks or system resources.

Caution must be exercised with this.
A thread cannot be reaped while it is running (as destroying the stack is potentially harmful).
Thus, threads usually should only be reaped after context switching out of them.
As well, if any threads were blocked waiting for a thread's exit code, the exit code of a dying thread must be stored and pushed to other threads (though other resources may be deallocated).

The [kernel thread](#the-kernel-thread) is an exception to this.

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
This disallows reordering of fields within the struct and mimics the alignment we would see within C structs.

The context switch function must retain the TCB pointers in order for the `prepare_thread` and `run_thread` function to properly schedule our threads.
However, internally, the context switch function must derefence these pointers to actually find the address of the stack.

### Notes on Scheduling

When calling a scheduler function, such as `scheduler_yield`, we must ensure that the scheduler knows of the correct running and waiting to run threads. There are two cases to consider.

#### Switching to a Previously Running Thread

When switching to a thread that was previously running, we are under the assumption that this thread was in the middle of running the `switch_threads` function.
Likewise, the thread we are switching from is left in the middle of the `switch_threads` function.
This makes scheduling easy.
The actual context switch will return the TCB of the thread we just came from.
As well, the new thread will be able to set itself as the running thread (and must).

#### Switching to a New Thread

In the case that a new thread is being started, it will not begin running in the middle of the `switch_threads` function.
Instead, it will begin to run the `prepare_thread` and then `run_thread` functions.
This gives the `run_thread` function the responsibility to enque our previously running thread in the scheduler and update the currently running thread.

#### Unique Threads

There are currently two 'special' thread within KidneyOS.

#### The Kernel Thread

The kernel thread is the code that runs before the threading system starts.
In order to streamline the transfer into the threading system, the kernel code is 'transformed' into a kernel thread (see [ThreadControlBlock::new_kernel_thread](./thread_control_block.rs)).

The only difference for now, is that the kernel thread's stack is not allocated by our allocator and thus must not be reaped the same as other threads.

Once the threading system is started, this kernel thread is immediately discarded and our kernel becomes entirely event-driven.

#### The Idle Thread

This thread simply yields to any other thread in the system and does nothing else.
This thread should _never_ be killed or blocked.
Doing so may leave the kernel and scheduler in a state where there is no other thread to run and thus would crash when trying to context switch.

To the kernel, this thread is opaque.
It should always be given the lowest possible priority to prevent it from being run unnecessarily.

## Context Switching

The function `switch_threads` is the public method for switching to a given thread.
This will ensure that thread's statuses are updated correctly.
The thread specified to switch to must be in the ready state.

Typically, you do not interact with `switch_threads` directly.
Rather, you use one of the scheduling functions provided in the mod file for the scheduling crate.

Context switching requires us to save the executing thread's state onto it's stack, switch to the new threads stack, and restore the new thread's state.
Luckily, the state we are required to save within the context switch is small (see notes on [calling conventions](#calling-conventions)).
Within [`context_switch.rs`](./context_switch.rs) you may find several small blocks of assembly that accomplish this task (such low level manipulation is impossible with Rust directly).
Each should be fairly self explanatory:

* `save_registers` has the job of placing the `$ebp, $ebx, $esi,` and `$edi` registers onto the current stack.
    As well, it moves value of the stack pointer (`$esp`) into `$ebp` to aide in finding arguments on the stack.
* `load_arguments` simply takes arguments off of well-known stack positions and places them into free registers.
* `switch_stacks` places the current stack pointer into the TCB for the currently executing thread before updating the stack pointer to that of the to-execute thread.
* `restore_registers` is just the reversal of `save_registers`.
    Notably, this is now moving values from the new stack.

The arguments provided to `context_switch` must be raw pointers to the TCB's that we wish to switch from and to.
See [Notes on Argument Types](#notes-on-argument-types) for information about these pointers.

### Calling Conventions

Unfortunately for us kernel developers, Rust does not have a standardized calling convention.
This is in an effort to allow optimizations at a low level.
For us, it is merely an inconvience.

Rust provides the ability to specify different ABI's for a given function.
This allows us to have a known calling convention where needed and prevent the compiler from changing this in the future.
We use the well known C convention requiring us to mark any function that we interact with through assembly to be marked `extern "C"`.
This enforces the Rust compiler to call into this function as if it were a C function.
[This chapter](https://aaronbloomfield.github.io/pdr/book/x86-32bit-ccc-chapter.pdf) of Aaron Bloomfield's book was used as a reference for the C calling convention.
Information about the specific registers can be found in [this document](https://www.eecg.utoronto.ca/~amza/www.mindsec.com/files/x86regs.html).

Briefly, the concern of our functions are:

* Before a function is called, the arguments are pushed onto the stack in reverse order (`foo(x, y, z)` is pushed as `z, y, x`).
* The `$eax, $ecx,` and `$edx` registers are saved by the caller and thus free for us to use.
* The callee must push `$ebp, $ebx, $esi,` and `$edi` onto the stack.
    * These must be restored on the return.
* Space for local variables can be allocated by decrementing the stack pointer (currently not needed for us).
* Return values are placed in `$eax` before returning and the stack must be returned to the same position as it was in entry.

It is obvious that we must declare `context_switch` and `prepare_thread` as `extern "C"` since they directly use the above assumptions.
However, `run_thread` must also be declared `extern "C"` despite being pure Rust.
This is because we must inform the compiler that this function is called by a C function (`prepare_thread`) so it knows how to access it's arguments.
Similarily, our `entry_function` for a thread must be `extern "C"` since we wish to be able to run any C program within this OS.

_Note:_ `prepare_thread` is not a 'real' function and must not be returned into.

## The Scheduler

The scheduler, found within [`scheduling`](./scheduling/), is the main attraction for students working with KidneyOS.
Implementing this will be an assignment that students must tackle.
The simple interface provides the kernel with three main touch points:

* `push` for adding a TCB into the scheduler.
* `pop` for retrieving the next TCB to run.
* `remove` for killing a thread within the scheduler.
    * This must use the thread ID rather than a TCB due to our (and Rust's) [ownership rules](#ownership-of-threads).

The intentionally bare interface provides ample opportunity for different schedulers behind the scenes.
Notably, students will likely be implementing a MLFQ or similar scheduler.
The kernel will not need to make any assumptions about scheduling order.

The implementation found within the kernel currently (the [FIFOScheduler](./scheduling/fifo_scheduler.rs)) is an incomplete scheduler that simply maintains a FIFO queue of threads.
