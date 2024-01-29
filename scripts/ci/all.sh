#!/usr/bin/env bash

# This will run all the things that CI does, but locally. This is helpful for
# now to see if everything works on a contributor's machine, since CI is not set
# up to run on any branches that aren't on the main repo.

# Requirements:
# - Nix

# bolt-check.yaml
nix-shell --pure --run "bolt gen project" \
&& nix-shell --pure --run "bolt check -g --validate-format" \

# bolt-test.yaml
&& nix-shell --pure --run "bolt init --yes ci" \
&& nix-shell --pure --run "bolt test" \
&& nix-shell --pure --run "k3d cluster delete rivet-ci"