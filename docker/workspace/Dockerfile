FROM debian:12.9

RUN apt-get update && apt-get install -y \
	curl \
	sudo \
	xz-utils \
	&& rm -rf /var/lib/apt/lists/*

RUN echo '#!/bin/bash\nexec nix-shell /app/shell.nix "$@"' > /usr/local/bin/nix-shell-wrapper && chmod +x /usr/local/bin/nix-shell-wrapper

RUN useradd -m nixuser && echo "nixuser ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/nixuser

USER nixuser

RUN curl -vL https://nixos.org/nix/install | sh

ENV USER nixuser
ENV PATH /home/nixuser/.nix-profile/bin:/home/nixuser/.nix-profile/sbin:/nix/var/nix/profiles/default/bin:/nix/var/nix/profiles/default/sbin:$PATH
ENV NIX_PATH /nix/var/nix/profiles/per-user/nixuser/channels

WORKDIR /app

COPY --chown=nixuser:nixuser shell.nix /app/

RUN nix-shell shell.nix --run "true"

ENTRYPOINT ["/usr/local/bin/nix-shell-wrapper"]

