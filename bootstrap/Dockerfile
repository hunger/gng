## SPDX-License-Identifier: GPL-3.0-or-later
## Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

# cSpell: ignore opencontainers

# Build bootstrap environment

# GH Actions will do the build and populate /nix as well as /gng/agent!

FROM scratch

ADD --chown=0:0 nix /
ADD --chown=0:0 usr gng /

LABEL org.opencontainers.image.authors="tobias.hunger@gmail.com"

ENTRYPOINT ["/gng/agent"]
