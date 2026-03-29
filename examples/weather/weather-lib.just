# Shared justfile helpers for the weather example.
# Import with: mod wlib '../../weather-lib.just' (adjust path as needed)

# Check that .env exists and has OPENWEATHER_API_KEY
[script('sh'), no-cd]
check-env:
    ENV_FILE="$(git rev-parse --show-toplevel)/examples/weather/.env"
    if [ -f "$ENV_FILE" ] && grep -q 'OPENWEATHER_API_KEY=' "$ENV_FILE"; then
        echo '  ✓ .env (OPENWEATHER_API_KEY set)'
    else
        echo '  ✗ .env — create examples/weather/.env with:'
        echo '    export OPENWEATHER_API_KEY=your_key_here'
        echo '    Get a free key at https://openweathermap.org/appid'
        exit 1
    fi
