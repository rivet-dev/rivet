#!/bin/sh
set -euf

yarn
yarn update-framer || true  # TODO: fix the actual error
npx next build && cp _redirects out/_redirects
