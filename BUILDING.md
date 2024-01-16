# Building KidneyOS

## With Nix

If you have a version of the [Nix](https://nixos.org) package manager with [flake support](https://nixos.org/manual/nix/unstable/command-ref/new-cli/nix3-flake.html) installed, you can run `nix develop ./nix#` from the root of the repository to build and enter a environment with all dependencies present. If you want to avoid downloading a few extra tools that aren't essential for building the project (but are useful for testing and debugging), you can run `nix develop ./nix#build` instead. Once the environment has been created, run `make build run-qemu` to build the project and boot the ISO with QEMU.

## With Docker

If you have [Docker](https://www.docker.com) or [Podman](https://podman.io) installed, you can use either to access the same environment created by the `nix develop ./nix#build` command used above. First, run `make -f Makefile.docker build-builder`. Once the image is done building and importing, run `make -f Makefile.docker run-builder` (if you are using Podman, you'll need to add `PODMAN=1` to the end of the command). This should drop you into an interactive session within a container in the same environment as above. Within this environment you can run `make build` to create the ISO. `make run-qemu` won't work within this environment since QEMU needs to be able to open windows, so you'll have to install QEMU on the host via another method.

## With Rustup

If you have [rustup](https://rustup.rs/) installed, you can create your environment manually via the following steps. This approach isn't recommended as it's harder to guarantee that your resulting environment will be identical to the ones above, in which versions are pinned. First, install the correct Rust toolchain by running `rustup install <channel> --profile <profile>`, where `<channel>` and `<profile>` are replaced by the respective values from the `rust-toolchain.toml` file in the root of the project. Then install Grub2, xorriso, and QEMU via your system package manager. Finally, run `make build run-qemu` to build the project and boot the ISO with QEMU.
