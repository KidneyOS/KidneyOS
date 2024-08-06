#!/usr/bin/env bash

set -euo pipefail

usage() {
  echo "Usage: $0 [-a | --arm]" >&2
  exit 2
}

case "$#" in
  0)
    system=x86_64-linux
    ;;
  1)
    case "$1" in
      -a | --arm)
        system=aarch64-linux
        ;;
      *)
        usage
        ;;
    esac
    ;;
  *)
    usage
    ;;
esac


# Build the container image. This script expects to be run from a host capable
# of building nix derivations for the selected system.
nix --extra-experimental-features nix-command \
  --extra-experimental-features flakes \
  build "./nix#packages.$system.kidneyos-builder"

# Import the result of the build into the Docker daemon.
docker load -i result
