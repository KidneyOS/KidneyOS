# Docker

This section describes how to build KidneyOS using Docker.

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

## Install Docker

If you already have Docker installed, you can skip this step. If you're on Linux, install the [`docker`](https://repology.org/project/docker/versions) package using your package manager. Otherwise, install Docker Desktop, which can be downloaded [here for MacOS](https://docs.docker.com/desktop/install/mac-install/) or [here for Windows](https://docs.docker.com/desktop/install/windows-install/).

## Run Container

Then execute the appropriate script. For Linux or MacOS, use the following:

```sh
scripts/run-container.sh
```

...or for Windows:

```powershell
scripts/run-container.ps1
```

<!-- TODO: implement the powershell script. -->

<!-- TODO: actually push the container to the GitHub container registry. -->

<!-- TODO: support some way of handling different container versions for once there are multiple releases. -->

This will pull the container from the GitHub container registry and start it. You will need to run the container with the script above each time you want to build KidneyOS.

## Install Host Tools

Since we can't run graphical tools inside the Docker container, we'll have to install the following two packages on the host:

- [`qemu`](https://repology.org/project/qemu/versions)
- [`bochs`](https://repology.org/project/bochs/versions) (Optional, but recommended as it is useful for debugging. See the corresponding ["Useful Tools" section](../useful-tools.md#bochs) for more information.)
