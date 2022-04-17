set dotenv-load := false
tarfile := "LOUDBOT_" + os() + "_" + arch() + ".tar"

# list just targets
default:
    @just -l

# format and run tests
test:
    @cargo fmt --all
    @cargo test

# build a release and package it up
release:
    @cargo build --release
    @tar cf {{tarfile}} -C target/release LOUDBOT PRUNE SEED
    @tar f {{tarfile}} -r MALCOLM CATS SEEDS SHIPS STAR_FIGHTING
    @gzip {{tarfile}}
    @echo "Release artifact in {{tarfile}}.gz"
