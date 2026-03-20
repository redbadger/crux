# Shared justfile helpers for Crux examples
# Import with: mod lib '../../lib.just' (adjust path as needed)

# Check if a tool is installed, report status (does not fail build)
[no-cd]
check-tool tool install_cmd:
    #!/usr/bin/env bash
    if command -v {{ tool }} &> /dev/null; then
        echo '  ✓ {{ tool }}'
    else
        echo '  {{ style("error") }}✗ {{ tool }}{{ NORMAL }} — install with: {{ install_cmd }}'
    fi

# Check if a file exists, report status (does not fail build)
[no-cd]
check-file file setup_cmd:
    #!/usr/bin/env bash
    if [ -f {{ file }} ]; then
        echo '  ✓ {{ file }}'
    else
        echo '  {{ style("error") }}✗ {{ file }}{{ NORMAL }} — create with: {{ setup_cmd }}'
    fi
