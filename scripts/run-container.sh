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

docker run $extra_flags --rm -it \
  -v "$project_root:/KidneyOS" \
  -w /KidneyOS \
  ghcr.io/kidneyos/kidneyos-builder:latest
