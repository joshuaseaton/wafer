# Copyright (c) 2025 Joshua Seaton
#
# Use of this source code is governed by a MIT-style
# license that can be found in the LICENSE file or at
# https://opensource.org/licenses/MIT

set quiet

# List available just recipes
list:
  just --justfile {{justfile()}} --list

# Check development dependencies
check-deps:
  just check-dep cargo  # Unlikely, but added for completeness
  just check-dep nu
  just check-dep wast2json

# Best to stick to POSIX-compliant shell before/while we check whether nushell
# is actually installed.
[private]
check-dep tool:
  #!/usr/bin/env sh
  (command -v {{tool}} >/dev/null && echo "{{tool}}: present") || \
    echo "{{tool}}: missing"

# Regenerate manifest of WASM spec tests
gen-spec-tests:
  #!/usr/bin/env nu
  cd third-party/github.com/WebAssembly/spec/test/core
  let wasts = ls | where name =~ .wast | get name
  cd {{justfile_directory()}}
  $wasts | to json | save --force spec-tests/wast.json

