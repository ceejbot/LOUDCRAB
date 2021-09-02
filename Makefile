#!/bin/bash
OSES = apple-darwin unknown-linux-gnu unknown-linux-musl
OS_TARGETS := $(foreach a,$(OSES),$(a)-arch)
EXECS = LOUDBOT PRUNE SEED
TEXTS = MALCOLM CATS SEEDS SHIPS STAR_FIGHTING
BOLD=\033[0;1;32m
NORMAL=\033[m
PWD := $(shell pwd)

all: release

%.tar: %-build
	@tar cf $@ -C target/x86_64-$*/release $(EXECS)
	@tar f $@ -r $(TEXTS)

%.tar.gz: %.tar
	@gzip $<

apple-darwin-build:
	@echo "Building $(BOLD)darwin$(NORMAL)..."
	@cross build --release --target x86_64-apple-darwin

unknown-linux-gnu-build:
	@echo "Building $(BOLD)gnu$(NORMAL)..."
	@cross build --release --target x86_64-unknown-linux-gnu

unknown-linux-musl-build:
	@echo "Building $(BOLD)musl$(NORMAL)..."
	docker run -v $(PWD):/volume --rm -it clux/muslrust cargo build --release --target x86_64-unknown-linux-musl

$(OS_TARGETS): %-arch: %.tar.gz
	@echo "    done.\n"

alpine: unknown-linux-musl-build

gnu: unknown-linux-gnu-build

release: $(OS_TARGETS)
	@mkdir -p releases
	@mv *.tar.gz releases/
	@echo "Tarballs in $(BOLD)./releases$(NORMAL)."

clean:
	rm -f *.tar *.gz releases/*
	rmdir releases

spotless: clean
	cargo clean

.PHONY: release clean spotless $(OS_TARGETS) $(OSES) alpine gnu
