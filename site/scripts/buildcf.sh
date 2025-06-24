#!/bin/sh
set -euf

yarn install
npx next build && cp _redirects out/_redirects
