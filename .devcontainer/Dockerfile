FROM mcr.microsoft.com/devcontainers/base:bookworm

RUN apt-get update \
    && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install \
        git-lfs \
        libssl-dev \
        pkg-config \
        build-essential \
        protobuf-compiler

# Create a folder for cargo target to use
RUN mkdir -p /target \
    && chown -R vscode:vscode /target
