#!/bin/bash

# This should be a makefile.
set -e

mkdir -p releases

cross build --target x86_64-unknown-linux-gnu --release
echo "Tarring up the gnu linux build..."
tar cf linux_release_x64.tar -C target/x86_64-unknown-linux-gnu/release/ LOUDBOT PRUNE SEED
tar f linux_release_x64.tar -r MALCOLM CATS SEEDS SHIPS STAR_FIGHTING
gzip linux_release_x64.tar
mv linux_release_x64.tar.gz releases/
echo "Linux release in ./releases/linux_release_x64.tar.gz"

cross build --target x86_64-unknown-linux-musl --release
echo "Tarring up the alpine linux build..."
tar cf alpine_release_x64.tar -C target/x86_64-unknown-linux-musl/release/ LOUDBOT PRUNE SEED
tar f alpine_release_x64.tar -r MALCOLM CATS SEEDS SHIPS STAR_FIGHTING
gzip alpine_release_x64.tar
mv alpine_release_x64.tar.gz releases/
echo "Linux release in ./releases/alpine_release_x64.tar.gz"

cargo build --release
echo "Tarring up the darwin build..."
tar cf darwin_release_x64.tar -C target/release/ LOUDBOT PRUNE SEED
tar f darwin_release_x64.tar -r MALCOLM CATS SEEDS SHIPS STAR_FIGHTING
gzip darwin_release_x64.tar
mv darwin_release_x64.tar.gz releases/
echo "Mac release built in ./releases/darwin_release_x64.tar.gz built."
