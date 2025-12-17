#!/usr/bin/env bash

## Bash script to start an interactive shell where you can
## test the TAB completion for the `completion_example` command.
## This requires that you have already setup bash-completion on your host:
## https://wiki.debian.org/Add%20Bash%20Completion

set -euo pipefail

exec bash --noprofile --rcfile <(cat <<'EOF'
PS1="## $ "
source <(cargo run --features completion --example completions completion bash)
alias completions='cargo run --features completion --example completions'

echo "## Entering sub-shell."
echo "## Test 'completion_example' and its TAB completion."
echo "## When done, press Ctrl-D, or type 'exit'."
echo
echo "## $ completions help"
completions help
echo
EOF
) -i
