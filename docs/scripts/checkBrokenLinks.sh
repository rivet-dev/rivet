#!/bin/sh
set -euf -o pipefail

URL=https://site-ckd.pages.dev/

# docker run --init -it -v `pwd`:/input lycheeverse/lychee --config /input/lychee.toml --base /input/src/pages '/input/src/pages/**/*.mdx'
# muffet --verbose --max-connections=16 --exclude='(hub.rivet.gg|tanks.rivet.game|discord.gg|twitter.com|github.com|cdn-cgi)' https://staging.rivet.gg
npx linkinator --config linkinator.config.json "$URL" > /tmp/linkinator.json
cat /tmp/linkinator.json | jq '.links[] | select(.status != 200)'
