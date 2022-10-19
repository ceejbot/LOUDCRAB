set dotenv-load := false
tarfile := "LOUDBOT_" + os() + "_" + arch() + ".tar"

# list just targets
default:
    @just -l

# format and run tests
test:
    @cargo fmt --all
    @cargo test

# generate docs and open them in a browser
docs:
    @cargo doc --no-deps --open

# build a release and package it up
release:
    @cargo build --release
    @tar cf {{tarfile}} -C target/release LOUDBOT PRUNE SEED
    @tar f {{tarfile}} -r MALCOLM CATS SEEDS SHIPS STAR_FIGHTING
    @gzip {{tarfile}}
    @echo "Release artifact in {{tarfile}}.gz"

# Set the crate version and tag the repo to match.
tag VERSION:
	#!/usr/bin/env bash
	status=$(git status --porcelain)
	if [ "$status" != ""  ]; then
		echo "There are uncommitted changes! Cowardly refusing to act."
		exit 1
	fi
	tomato set package.version {{VERSION}} Cargo.toml
	# update the lock file
	cargo check
	git commit Cargo.toml Cargo.lock -m "v{{VERSION}}"
	git tag "v{{VERSION}}"
	echo "Release tagged for version v{{VERSION}}"
