# macOS (ARM)

## Install a Cross-Compiler

Since KidneyOS is built for x86, in order to target the correct architecture during compilation, you need to use a [cross compiler](https://en.wikipedia.org/wiki/Cross_compiler).
We suggest using [this one](https://github.com/messense/homebrew-macos-cross-toolchains) which can be installed using Homebrew.

```sh
brew tap messense/macos-cross-toolchains
brew install i686-unknown-linux-gnu
```

## Download Dependencies

There are no pre-built binaries of Grub 2 for ARM macOS so you will need to build it yourself.
In order to do this, make sure the following dependencies are installed.

```sh
brew install objconv gawk # Required to build by grub
brew install autoconf automake pkg-config
brew install xorriso # Invoked to create the ISO
```

[//]: # (https://gist.github.com/emkay/a1214c753e8c975d95b4?permalink_comment_id=4612920#gistcomment-4612920)
One final step before building is to alias `gawk` because the default `awk` in macOS doesn't work nicely with the grub installation scripts.

```sh
alias awk=gawk
```

## Build and Install Grub
The first step is to clone the grub repository.

```sh
git clone git://git.savannah.gnu.org/grub.git
cd grub
```

```sh
./bootstrap # Configures some things.
./autogen.sh
```

```sh
mkdir build
cd build
../configure --disable-werror \
    TARGET_CC=i686-unknown-linux-gnu-gcc \
    TARGET_OBJCOPY=i686-unknown-linux-gnu-objcopy \
    TARGET_STRIP=i686-unknown-linux-gnu-strip \
    TARGET_NM=i686-unknown-linux-gnu-nm \
    TARGET_RANLIB=i686-unknown-linux-gnu-ranlib \
    --target=i686-unknown-linux-gnu
```

After this step, you should've successfully built grub.
You *must* install grub, otherwise certain bootloader mod files will silently be excluded from your ISO and QEMU will give you CD ROM Code `0004` or `0009`.
This writes the grub binaries to `/usr/local/bin` and requires root permissions.

```sh
sudo make install
```

You should now be able to directly run `make run-qemu` from your terminal!
