#!/bin/bash
OSES = apple-darwin unknown-linux-gnu unknown-linux-musl
OS_TARGETS := $(foreach a,$(OSES),$(a)-arch)
EXECS = LOUDBOT PRUNE SEED
TEXTS = MALCOLM CATS SEEDS SHIPS STAR_FIGHTING
BOLD=\033[0;1;32m
NORMAL=\033[m

all: release

%.tar: %-build
	@tar cf $@ -C target/x86_64-$*/release $(EXECS)
	@tar f $@ -r $(TEXTS)

%.tar.gz: %.tar
	@gzip $<

%-build:
	@echo "Building $(BOLD)$*$(NORMAL)..."
	@cross build --release --target x86_64-$*

$(OS_TARGETS): %-arch: %.tar.gz
	@echo "    done.\n"

release: $(OS_TARGETS)
	@mkdir -p releases
	@mv *.tar.gz releases/
	@echo "Tarballs in $(BOLD)./releases$(NORMAL)."

alpine: unknown-linux-musl-arch

gnu: unknown-linux-gnu-arch

clean:
	rm -f *.tar *.gz releases/*
	rmdir releases

spotless: clean
	cargo clean

.PHONY: release clean spotless $(OS_TARGETS) $(OSES) alpine gnu
