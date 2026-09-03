default: build

all: test

test: build
	cargo test

build:
	stellar contract build
	@ls -l target/wasm32v1-none/release/*.wasm

fmt:
	cargo fmt --all

lint:
	cargo clippy --lib --target wasm32v1-none -- -D warnings

# Deploys the release WASM built by `make build` to Stellar testnet.
# Requires a funded testnet identity configured locally first, e.g.:
#   stellar keys generate testnet-deployer --network testnet --fund
# Override the account alias with: make deploy-testnet SOURCE_ACCOUNT=your-alias
SOURCE_ACCOUNT ?= testnet-deployer

deploy-testnet: build
	@echo "Deploying to testnet using source account alias '$(SOURCE_ACCOUNT)'."
	@echo "If this account isn't configured yet, run:"
	@echo "  stellar keys generate $(SOURCE_ACCOUNT) --network testnet --fund"
	stellar contract deploy \
		--wasm target/wasm32v1-none/release/stellar_grants.wasm \
		--network testnet \
		--source-account $(SOURCE_ACCOUNT) \
		--alias stellar_grants

clean:
	cargo clean
