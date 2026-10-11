#!/usr/bin/env bash
# Fail if any `{{#include path:anchor}}` in the book names a file or anchor that does not exist.
#
# mdBook only warns about a missing anchor, and includes nothing, so a renamed
# or deleted `ANCHOR` in an example silently blanks a code listing in the book.
# Paths resolve relative to the including page; line-range includes (`:N`,
# `:N:M`) have no anchor to check, so they only need the file to exist.
set -euo pipefail

cd "$(dirname "$0")/.."

error=$'\e[1;31m'
normal=$'\e[0m'

status=0
checked=0
while IFS= read -r hit; do
    [ -n "$hit" ] || continue
    page=${hit%%:*}
    spec=$(printf '%s' "${hit#*:}" | sed -E 's/^[{][{]#(rustdoc_)?include +//; s/ *[}][}]$//')
    path=${spec%%:*}
    anchor=
    if [ "$path" != "$spec" ]; then
        anchor=${spec#*:}
    fi
    checked=$((checked + 1))
    target="$(dirname "$page")/$path"
    if [ ! -f "$target" ]; then
        echo "  ${error}✗ $page includes $path, which does not exist${normal}"
        status=1
    elif [[ "$anchor" =~ ^[A-Za-z_][A-Za-z0-9_-]*$ ]] && ! grep -qE "ANCHOR: ${anchor}( |\$)" "$target"; then
        echo "  ${error}✗ $page includes $path:$anchor, but $path has no such anchor${normal}"
        status=1
    fi
done <<INCLUDES
$(grep -rHoE '[{][{]#(rustdoc_)?include [^}]*[}][}]' src --include='*.md' || true)
INCLUDES

if [ "$checked" -lt 100 ]; then
    echo "  ${error}✗ only found $checked includes, expected at least 100. Has the book moved?${normal}"
    status=1
fi

if [ "$status" -eq 0 ]; then
    echo "  ✓ $checked includes resolve"
fi
exit $status
