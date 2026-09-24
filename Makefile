# Always build with --arch v0: Anchor 1.2 defaults to SBPF v3, which neither
# current mainnet/devnet deployment nor LiteSVM 0.10 accepts yet.
.PHONY: build test deploy-devnet

build:
	anchor build --arch v0

test: build
	cargo test

deploy-devnet: build
	anchor deploy --provider.cluster devnet

demo:
	cargo run -p latch-demo
