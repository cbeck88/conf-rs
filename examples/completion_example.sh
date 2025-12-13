#!/usr/bin/env bash

## Bash script to start an interactive shell where you can
## test the TAB completion for the `completion_example` command.
## This requires that you have already setup bash-completion on your host:
## https://wiki.debian.org/Add%20Bash%20Completion

set -euo pipefail

exec bash --noprofile --rcfile <(cat <<'EOF'
PS1="## $ "
source <(cargo run --features completion --example completion_example completion bash)
alias completion_example='cargo run --features completion --example completion_example'

echo "## Entering sub-shell."
echo "## Test 'completion_example' and its TAB completion."
echo "## When done, press Ctrl-D, or type 'exit'."
echo
echo "## $ completion_example help"
(set -x; completion_example help)
echo
EOF
) -i
