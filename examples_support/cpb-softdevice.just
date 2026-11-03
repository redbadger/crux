# Shared by the CPB firmware examples (counter/cpb, counter-http/cpb).
# Import with: import '../../../examples_support/cpb-softdevice.just'
#
# A board starts the application right after its SoftDevice: 0x26000 with S140 6.x
# (cargo feature sd-v6) and 0x27000 with S140 7.x (cargo feature sd-v7).

# The board's bootloader drive. Override with CPB_DRIVE=/path/to/CPLAYBTBOOT.
drive := env("CPB_DRIVE", if os() == "macos" { "/Volumes/CPLAYBTBOOT" } else { "/media/" + env("USER", "") + "/CPLAYBTBOOT" })
uf2_family := "0xADA52840"
uf2_tools := "target/uf2-tools"

# print the board's SoftDevice and the memory layout that matches it (build and flash nothing)
[script('bash')]
board:
    set -euo pipefail
    echo '{{ style("command") }}board:{{ NORMAL }}'
    {{ just_executable() }} _sd-layout auto report > /dev/null

# (internal) print v6 or v7 on stdout. sd is auto (read INFO_UF2.TXT on the board), v6 or v7.
[script('bash')]
_sd-layout sd="auto" report="":
    set -euo pipefail
    sd='{{ sd }}'
    if [ "$sd" = auto ]; then
        info='{{ drive }}/INFO_UF2.TXT'
        if [ ! -f "$info" ]; then
            echo '  {{ style("error") }}✗ {{ drive }} is not mounted{{ NORMAL }}: double-press the reset button, set CPB_DRIVE, or pass v6 or v7 as the sd argument' >&2
            exit 1
        fi
        line=$(grep -m1 '^SoftDevice:' "$info" || true)
        version=$(printf '%s\n' "$line" | sed -n 's/^SoftDevice: S140 \([0-9][0-9.]*\).*/\1/p')
        case "$version" in
            6.*) sd=v6 ;;
            7.*) sd=v7 ;;
            *)
                echo "  {{ style("error") }}✗ unsupported SoftDevice{{ NORMAL }} in $info: '${line:-no SoftDevice line}' (expected S140 6.x or 7.x). Pass v6 or v7 as the sd argument to choose." >&2
                exit 1
                ;;
        esac
        if [ -n '{{ report }}' ]; then
            if [ "$sd" = v6 ]; then origin=0x26000; else origin=0x27000; fi
            echo "  SoftDevice S140 $version: layout $sd (flash origin $origin)" >&2
        fi
    elif [ "$sd" != v6 ] && [ "$sd" != v7 ]; then
        echo "  {{ style("error") }}✗ unknown sd '$sd'{{ NORMAL }}: use auto, v6 or v7" >&2
        exit 1
    fi
    echo "$sd"

# (internal) print the cargo flags for a layout (v6 is the default feature, so it needs none)
[script('bash')]
_sd-features sd="v6":
    set -euo pipefail
    case '{{ sd }}' in
        v6) ;;
        v7) echo '--no-default-features --features sd-v7' ;;
        *) echo "  {{ style("error") }}✗ unknown sd '{{ sd }}'{{ NORMAL }}: use v6 or v7" >&2; exit 1 ;;
    esac

# (internal) fetch Microsoft's uf2conv.py into target/ (not committed)
[script('bash')]
_uf2-tools:
    set -euo pipefail
    mkdir -p {{ uf2_tools }}
    for file in uf2conv.py uf2families.json; do
        [ -f {{ uf2_tools }}/$file ] || curl -fsSL -o {{ uf2_tools }}/$file https://raw.githubusercontent.com/microsoft/uf2/master/utils/$file
    done
