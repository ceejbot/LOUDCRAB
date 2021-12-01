#!/bin/bash
OSES = aarch64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-musl x86_64-unknown-linux-gnu
TARS := $(foreach a,$(OSES),built/$(a).tgz)

ARM = x86_64-apple-darwin aarch64-apple-darwin aarch64-unknown-linux-gnu
ARM_TARGETS := $(foreach a,$(ARM),built/loudbot-$(a).tgz)

X86 = x86_64-unknown-linux-musl x86_64-unknown-linux-gnu
X86_TARGETS := $(foreach a,$(X86),built/loudbot-$(a).tgz)

EXECS = LOUDBOT PRUNE SEED
TEXTS = MALCOLM CATS SEEDS SHIPS STAR_FIGHTING

BOLD=\033[0;1;32m
NORMAL=\033[m
PWD := $(shell pwd)

all: container dirs $(TARS)
	@echo "Tarballs in $(BOLD)./built$(NORMAL)."

container:
	@docker build -t loudcross - < Dockerfile.builds

check-box:
    @docker run -v $(PWD):/src -w /src --rm -it loudcross /bin/bash

arm: container dirs $(ARM_TARGETS)
	@echo "Tarballs for ARM architectures & Intel Darwin in $(BOLD)./built/$(NORMAL)."
	@echo "Build for x86 Linux on an Intel Mac!"

x86: container dirs $(X86_TARGETS)
	@echo "Tarballs for Linux X86 hosts in $(BOLD)./built$(NORMAL)."
	@echo "Build for ARM on an M1 Mac!"

dirs:
	@mkdir -p ./built

built/loudbot-%.tgz: %
	@tar cf $*.tar -C target/$*/release $(EXECS)
	@tar f $*.tar -r $(TEXTS)
	@gzip $*.tar
	@mv $*.tar.gz built/loudbot-$*.tgz
	@echo "    done.\n"

x86_64-apple-darwin:
	@echo "Building $(BOLD)x86 darwin$(NORMAL)..."
	@cargo build --release --target x86_64-apple-darwin

aarch64-apple-darwin:
	@echo "Building $(BOLD)m1 darwin$(NORMAL)..."
	@cargo build --release --target aarch64-apple-darwin

aarch64-unknown-linux-gnu:
	@echo "Building $(BOLD)graviton$(NORMAL)..."
	@docker run -v $(PWD):/src -w /src --rm -it loudcross cargo build --release --target aarch64-unknown-linux-gnu

x86_64-unknown-linux-gnu:
	@echo "Building $(BOLD)gnu$(NORMAL)..."
	@# does not work on m1: fails to run this in a container
	@#cross build --release --target x86_64-unknown-linux-gnu
	@docker run -v $(PWD):/src -w /src --rm -it loudcross cargo build --release --target x86_64-unknown-linux-gnu

x86_64-unknown-linux-musl:
	@echo "Building $(BOLD)musl$(NORMAL)..."
	@docker run -v $(PWD):/src -w /src --rm -it clux/muslrust cargo build --release --target x86_64-unknown-linux-musl

m1: built/loudbot-aarch64-apple-darwin.tgz

spaceheater: built/loudbot-x86_64-apple-darwin.tgz

alpine: built/loudbot-x86_64-unknown-linux-musl.tgz

gnu: built/loudbot-x86_64-unknown-linux-gnu.tgz

graviton: built/loudbot-aarch64-unknown-linux-gnu.tgz

clean:
	rm -f *.tar *.gz built/*
	rmdir built

spotless: clean
	cargo clean

.PHONY: clean spotless $(OS_TARGETS)
