# Useful Tools

This section contains info about a few tools that might be helpful as you work on KidneyOS.

## GDB

GDB is a debugger which supports a variety of programming languages. You can debug an instance of KidneyOS running inside QEMU by first starting the virtual machine with:

```sh
make run-qemu-gdb
```

The QEMU process will wait for GDB to attach before starting the OS.

The root of the KidneyOS repository contains a `.gdbinit` file, which makes a variety of useful configuration tweaks. Before GDB will use this file though, you'll likely need to add the following to your user-level gdbinit file (which is located at `~/.config/gdb/gdbinit` on Linux and MacOS):

```gdbinit
set auto-load safe-path .../KidneyOS # Replace this with the path to your repository.
```

Then, run the following from the root of the KidneyOS repository. (If you built KidneyOS with Docker and are running QEMU on your host, and `rust-gdb` is not installed on your host system, you can also use plain `gdb`, it just won't have the Rust-specific pretty printers added by `rust-gdb`, which is just a wrapper script.)

```sh
rust-gdb
```

This will start GDB. After you hit the enter key once, GDB should attach to QEMU and resume execution of the OS until it reaches the `_start` function defined in `trampoline/src/lib.rs`. From this point onwards, you should be able to use GDB as usual.

## Bochs

[Bochs](https://bochs.sourceforge.io) is a x86 emulator. (Note that it is a GUI program, meaning if you are building KidneyOS using Docker, you'll have to install it separately on your host system.) It has some, though not all, of the debugging features that GDB does, but it also has additional features which are more specific to OS development that GDB doesn't have. Some examples include:

- It has an indicator which displays the current CPU mode.
- You can dump sections of memory based on either physical or linear addresses.
- It has windows for viewing the contents of the global and interrupt descriptor tables, and the memory page tables.

You can run KidneyOS with Bochs by running:

```sh
make run-bochs
```

The `bochs_break!` macro (which is defined in `shared/src/macros.rs`) may be of use when running KidneyOS with Bochs.
