#!/bin/sh
set -euf

yarn install
yarn build
cp _redirects out/_redirects
