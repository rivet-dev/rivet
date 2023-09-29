FROM debian:12.1-slim

# Prevent apt from using the OpenSSL installation from /nix/store if mounted
ENV LD_LIBRARY_PATH=/lib:/usr/lib:/usr/local/lib

RUN DEBIAN_FRONTEND=noninteractive apt-get update -y && apt-get install -y --no-install-recommends ca-certificates

