# Always build with --arch v0: Anchor 1.2 defaults to SBPF v3, which neither
# current mainnet/devnet deployment nor LiteSVM 0.10 accepts yet.
.PHONY: build test deploy-devnet e2e-local

build:
	anchor build --arch v0

test: build
	cargo test

deploy-devnet: build
	anchor deploy --provider.cluster devnet

demo:
	cargo run -p latch-demo

# Full SDK lifecycle against a fresh local validator (program preloaded at
# genesis). Needs `make build` and a built clients/ts workspace.
e2e-local:
	./scripts/e2e-local.sh
