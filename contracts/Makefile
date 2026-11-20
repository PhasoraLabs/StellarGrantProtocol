# Convenience targets matching the commands documented in ContributionGuide.md
# and README.md. Each target delegates to the stellar-grants crate's own
# Makefile so the build logic lives in one place.

CRATE_DIR := contracts/stellar-grants

.PHONY: build test lint fmt deploy-testnet

build:
	$(MAKE) -C $(CRATE_DIR) build

test:
	$(MAKE) -C $(CRATE_DIR) test

lint:
	$(MAKE) -C $(CRATE_DIR) lint

fmt:
	$(MAKE) -C $(CRATE_DIR) fmt

deploy-testnet:
	$(MAKE) -C $(CRATE_DIR) deploy-testnet
