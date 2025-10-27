#!/bin/bash -x

# This is meant to be run at the project root
# tests/generate-reverse.sh

YTSC="${YTSC:-ytsubconverter}"

ASS_TEST_DIR="tests/ass"

cleanup_test() {
    rm -rfv test/ass/*.reverse.ass
}


cleanup_test

for file in "$ASS_TEST_DIR"/*.ytt; do
    $YTSC $file
done