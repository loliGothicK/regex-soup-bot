#!/bin/sh

case $(uname -m) in
  # linux/amd64
  "x86_64"  ) cp /work/artifacts/x86_64-unknown-linux-gnu      /usr/local/bin/regexsoup ;;
  # linux/arm/v7
  "armv7l"  ) cp /work/artifacts/armv7-unknown-linux-gnueabihf /usr/local/bin/regexsoup ;;
  # linux/arm64/v8
  "aarch64" ) cp /work/artifacts/aarch64-unknown-linux-gnu     /usr/local/bin/regexsoup ;;
  * ) exit 1 # we do not expect other platform
esac
