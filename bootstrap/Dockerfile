## SPDX-License-Identifier: GPL-3.0-or-later
## Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

# cSpell: ignore opencontainers

# Build bootstrap environment

# GH Actions will do the build and populate /nix as well as /gng/agent!

FROM scratch

ADD nix-store.tar.zst /
ADD gng-agent.tar.zst /

ENV PATH /usr/bin:/nix/var/profiles/bootstrap/bin

ENTRYPOINT ["/gng/agent"]
