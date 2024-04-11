# Package Manager/From Source

This section describes how to build KidneyOS by installing dependencies using a package manager or building them from source.

> ❗ This method is only supported on `x86_64-linux` and `aarch64-linux` hosts, or in WSL.

> ⚠️ The instructions that follow are designed to be as platform-agnostic as possible. However, the names of packages are not necessarily the same across different package managers. When mentioning package names, we will provide links to [Repology](https://repology.org/), which you should consult to determine the specific name used by your package manager.

<!-- [> clone](clone.md) -->
<!-- BEGIN mdsh -->
## Clone Repository

Clone the repository and `cd` into the resulting directory. (Depending on how your instructor wants you to submit your work, they may have given you an alternate repository URL. If so, use that URL instead of the one below.)

```sh
git clone https://github.com/KidneyOS/KidneyOS
cd KidneyOS
```

<!-- TODO: Provide instructions for checking out the appropriate branch for once we have stable, tagged versions. -->
<!-- END mdsh -->

## Install Rust Toolchain

Next, we will install the specific version of the Rust toolchain used by KidneyOS. We will do this using rustup. You can install it by running the command on the [rustup website](https://rustup.rs/), or by installing the [`rustup`](https://repology.org/project/rustup/versions) package with your package manager. Once you've done so, run the following from the root of the KidneyOS repository:

```sh
rustup override set nightly-2024-01-04-i686-unknown-linux-gnu
```

Running `cargo --version` should now print:

```
cargo 1.77.0-nightly (add15366e 2024-01-02)
```

## Install `i686-unknown-linux` Build Tools

A non-exhaustive list of the binaries you'll need to install includes `i686-unknown-linux-gnu-ld` and `i686-unknown-linux-gnu-objcopy`. Some package managers may have a package for these. If yours doesn't, you may have to build them from source. Admittedly, this section is quite light on details, since the process is highly platform-specific. This may be updated with more details in the future. <!-- TODO: Provide more detailed instructions on how to do this, or a link to somewhere which does? -->

## Install Other Tools

We'll also need to install the following packages:

- [`grub`](https://repology.org/project/grub/versions)
  - Make sure it's Grub version 2.
  - If you're on `aarch64-linux`, you'll need to make sure you have the right libraries to build `i386-pc` ISOs. This will be highly distro-specific specific. You may have to build Grub from source.
- [`qemu`](https://repology.org/project/qemu/versions)
- [`xorriso`](https://repology.org/project/xorriso/versions)
- [`bochs`](https://repology.org/project/bochs/versions)
  - Optional, but recommended as it is useful for debugging. See the corresponding ["Useful Tools" section](../useful-tools.md#bochs) for more information.
