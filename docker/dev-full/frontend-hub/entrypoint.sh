#!/bin/bash
set -e

npm i -g corepack
corepack enable

# Install packages
cd /app
yarn install

# Start dev server
cd /app/frontend/apps/hub
yarn dev --host 0.0.0.0 --port 5080
