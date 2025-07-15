#!/bin/sh
set -euf

# TODO: Switch this to StackBlitz soon
CWD=$(pwd)
if [ ! -d "../actor-core" ]; then
    git clone --depth=1 --branch 04-04-fix_inspector https://github.com/rivet-gg/rivetkit.git ../actor-core
fi
cd ../actor-core 
yarn install
yarn build
