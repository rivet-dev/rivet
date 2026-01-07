#!/bin/bash
set -e

# Runtime environment variable substitution for Vite apps
# This script replaces placeholder values in the built JS files with actual environment variables

# Directory containing built files
HTML_DIR="/usr/share/nginx/html"

# Function to replace placeholder with environment variable value
replace_env_var() {
    local placeholder=$1
    local env_var=$2
    local value="${!env_var}"

    if [ -n "$value" ]; then
        echo "Replacing $placeholder with value from $env_var"
        # Use find + xargs for better performance with many files
        find "$HTML_DIR" -type f \( -name "*.js" -o -name "*.html" \) -print0 | \
            xargs -0 -r sed -i "s|$placeholder|$value|g"
    fi
}

# Replace all VITE_* environment variable placeholders
# Placeholders are in format: https://VARNAME.placeholder.rivet.gg
replace_env_var "https://VITE_APP_API_URL.placeholder.rivet.gg" "VITE_APP_API_URL"
replace_env_var "https://VITE_APP_CLOUD_API_URL.placeholder.rivet.gg" "VITE_APP_CLOUD_API_URL"
replace_env_var "https://VITE_APP_ASSETS_URL.placeholder.rivet.gg" "VITE_APP_ASSETS_URL"
replace_env_var "pk_placeholder_clerk_key" "VITE_APP_CLERK_PUBLISHABLE_KEY"
replace_env_var "https://VITE_APP_SENTRY_DSN.placeholder.rivet.gg/0" "VITE_APP_SENTRY_DSN"

echo "Environment variable substitution complete"

# Execute the CMD
exec "$@"
