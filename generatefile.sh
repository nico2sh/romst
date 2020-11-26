#!/bin/bash

FILENAME=$1
SIZE=$2

[ -z "$SIZE" ] && SIZE=2048

head -c $SIZE </dev/urandom >$FILENAME

CRC=$(crc32 $FILENAME)
SHA1=$(shasum -p $FILENAME)
SHA1=${SHA1%% *}

echo "<rom name=\"$FILENAME\" size=\"$SIZE\" crc=\"$CRC\" SHA1=\"$SHA1\" />"
