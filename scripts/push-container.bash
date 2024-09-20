#!/usr/bin/env bash

set -euo pipefail

if test "$#" -ne 0; then
  echo "Usage: $0" >&2
  exit 2
fi

# You need to be logged in to the GitHub container registry and have sufficient
# permissions within the KidneyOS organization for this to work.
docker push ghcr.io/kidneyos/kidneyos-builder
