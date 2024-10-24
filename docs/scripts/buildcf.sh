#!/bin/sh
# Clone modules repository to parent directory
cd ..
git clone https://github.com/rivet-gg/modules.git --depth=1 --branch main
cd repo
yarn
next build && cp _redirects out/_redirects