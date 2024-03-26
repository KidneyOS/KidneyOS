# Building

In this section, we'll walk you through building a KidneyOS ISO from source on your computer.

## Creating a Build Environment

First, we need to prepare an environment with the necessary dependencies. There are multiple approaches you can take. (Your instructor may have provided guidance on which of these options to choose.) **After choosing _one_ of these methods and completing the steps, return to this page and proceed to the next section.**

- You can run a prepared Ubuntu virtual machine image with all dependencies pre-installed using Virtual Box. If your host system is reasonnably powerful, and you're willing to set up your IDE/editor inside the virtual machine, this option is likely the simplest. Click [here](./building/virtualbox.md) for instructions.
- You can run the build tools inside a Docker container. This method may have slightly less overhead than the previous one (especially on Linux), but it is more complicated as some graphical development tools can't be run inside Docker, so you'll still have to install some things on your host system. Click [here](./building/docker.md) for instructions.
- You can install the dependencies manually on your host system (or in WSL) using your package manager and building from source when necessary. If your host system is `x64_64-linux` or `aarch64-linux`, or you have a [WSL](https://learn.microsoft.com/en-us/windows/wsl) installation on your Windows host, and you're familiar with how your package manager works, this approach is likely the best. Be aware that this option will probably require the most work, and that the packages available in your system's package manager may not have the same versions as those that would be installed when using the other options, which may lead to issues. Click [here](./building/package-manager-from-source.md) for instructions.
- You can use a Nix devshell. The [Nix package manager](https://nixos.org) is a somewhat advanced tool, but if you're already familiar with it, or are willing to spend a little time learning it, this option is likely the most reliable. The same platform restrictions as with the previous option also apply here. The virtual machine image and container used in the first two two approaches are built using the same Nix flake used by this option, meaning all three methods are guaranteed to have the identical versions of the dependencies. Click [here](./building/nix.md) for instructions.

## Building KidneyOS

Now that you have a build environment ready, simply run:

```sh
make
```

...from within your build environment, at the root of the KidneyOS repository. This will compile the various pieces of KidneyOS, and produce an ISO at `build/kidneyos.iso`.

## Running KidneyOS

To run the ISO you've just built, run:

```sh
make run-qemu
```

There are two things to note here:

1. If you're building KidneyOS with Docker, this command should be run on the host system, not in the container build environment.
2. If you're building KidneyOS with any of the other methods, you don't have to run `make` first after modifying the source code. `make run-qemu` will also automatically rebuild the ISO if any of the KidneyOS source files have changed.
