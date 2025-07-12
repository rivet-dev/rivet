#!/bin/sh
set -euf

CWD=$(pwd)
if [ ! -d "../actor-core" ]; then
    git clone --depth=1 --branch 04-04-fix_inspector https://github.com/rivet-gg/rivetkit.git ../actor-core
fi
cd ../actor-core 
yarn install
yarn build
cd $CWD
yarn install
