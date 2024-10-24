#!/usr/bin/env bash

set -euo pipefail

# pipefail ensures we fail if mdsh does, while the `grep ... || true` silences
# some of the unnecessary output, without failing if there isn't any.
find . -name '*.md' -print0 \
  | xargs -0 dirname \
  | sort -u \
  | xargs -I '{}' sh -c 'cd {} && mdsh --frozen -i *.md' \
  |& grep -Ev "^(Using input=|\[> )" || true
