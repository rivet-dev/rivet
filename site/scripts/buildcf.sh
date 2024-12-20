#!/bin/sh
set -euf

yarn
yarn update-framer
npx next build && cp _redirects out/_redirects
