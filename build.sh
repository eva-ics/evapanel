#!/usr/bin/env bash

ARCH=$(uname -m)
ARCH=$(echo "${ARCH}" | sed 's/arm.*/arm/g')
ARCH=$ARCH cargo build || exit 1
strip target/release/evapanel
echo $ARCH
