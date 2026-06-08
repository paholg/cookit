#!/usr/bin/env bash
set -euo pipefail

direnv allow
nix develop --command diesel database setup
