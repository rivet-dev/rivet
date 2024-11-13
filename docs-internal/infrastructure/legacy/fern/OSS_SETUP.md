# Fern OSS Setup

To install fern, first clone the repo and check out the branch

```shell
$ git clone https://github.com/rivet-gg/fern
$ cd fern
$ git checkout max/remove-headers
```

Then, follow the instructions in SETUP.md and CONTRIBUTING.md to compile fern

```shell
yarn
yarn compile
yarn dist:cli:dev
docker image ls | grep fern
docker builder prune
yarn workspace @fern-typescript/sdk-generator-cli run dockerTagVersion:browser 999.999.999
yarn workspace @fern-api/openapi-generator run dockerTagVersion 999.999.999
```

Finally, run this with the path to the fern repo, say:

```shell
FERN_REPO_PATH=~/fern ./oss/scripts/fern/gen.sh
```

