#!/bin/bash
set -e

# Install packages
cd /app
yarn install

# Start dev server
#
# Set base to /ui since this is where the UI is hosted in the dev server
cd /app/frontend/apps/hub
yarn dev --base=/ui
