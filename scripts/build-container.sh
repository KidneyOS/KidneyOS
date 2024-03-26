#!/usr/bin/env bash

set -euo pipefail

usage() {
  echo "Usage: $0 [-p | --podman]" 2>&1
  exit 2
}

extra_flags=
case "$#" in
  0)
    ;;
  1)
    case "$1" in
      -p | --podman)
        extra_flags=--userns=keep-id
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

project_root=$(dirname "$(dirname "$(realpath "${BASH_SOURCE[0]}")")")

docker container ls -a --format "{{.Names}}" \
  | grep kidneyos-builder-builder \
  || docker create $extra_flags --name kidneyos-builder-builder \
     -t -v "$project_root:/KidneyOS" \
     -w /KidneyOS nixos/nix:latest \
     bash -c 'cp "$(nix --extra-experimental-features flakes --extra-experimental-features nix-command build --no-link --print-out-paths ./nix#kidneyos-builder)" kidneyos-builder.tar.gz'
docker start -ai kidneyos-builder-builder
docker load -i kidneyos-builder.tar.gz
