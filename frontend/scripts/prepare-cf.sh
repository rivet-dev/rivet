#!/bin/sh
set -euf

CWD=$(pwd)
git clone --depth=1 --branch 04-04-fix_inspector git@github.com:rivet-gg/actor-core.git ../actor-core
cd ../actor-core 
yarn install
yarn build
cd $CWD
yarn install