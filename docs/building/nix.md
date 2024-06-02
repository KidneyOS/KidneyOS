# Nix

This section describes how to build KidneyOS using the Nix package manager.

> ❗ This method is only supported on `x86_64-linux` and `aarch64-linux` hosts. You can still use it indirectly on other host systems via [WSL](https://learn.microsoft.com/en-us/windows/wsl) or a virtual machine, but installing WSL or seting up a virtual machine will be up to you. If you chose to do this, all of the following commands should be run in the WSL or virtual machine environment.

## Installing Nix

If you already have Nix installed, you can skip this step. Note that the Nix package manager can be installed alongside your system's existing package manager. You don't have to uninstall your existing package manager. Instructions for installing Nix can be found [here](https://nixos.org/download). Both a multi-user installation or a single-user installation will work.

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

## Starting the "devshell"

Now that we have Nix installed and the repository cloned, we can build and run a shell containing all the dependencies (known as a "devshell" in Nix terminology). Run the following command:

```sh
nix --extra-experimental-features flakes \
    --extra-experimental-features nix-command \
    develop ./nix#
```

(The `--extra-experimental-features` flags may be unnecessary depending on your Nix version and configuration.) Note that it may take a while for everything to be downloaded and built. Once it finishes, you should be in a shell containing everything needed to build and run KidneyOS. You will need to run the command each time you want to enter a shell in which you can build KidneyOS.

### Direnv

This section is optional. If you'd like to avoid having to run the command in the prior section manually each time, you can accomplish this by using [direnv](https://github.com/direnv/direnv) with [nix-direnv](https://github.com/nix-community/nix-direnv). After following the installation instructions for both projects, run:

```sh
direnv allow
```

...from the root of the KidneyOS repository. Now, every time you `cd` into the KidneyOS repository, the "devshell" environment will automatically be made available, without you having to run any commands.

> If you use an IDE, installing its direnv extension/plugin (if one is available) and setting this up to use the environment defined in the KidneyOS repository may result in better error messages and autocompletion. The VSCode extension is available [here](https://marketplace.visualstudio.com/items?itemName=mkhl.direnv).
