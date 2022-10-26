# Warp Contract
[![warp contract on crates.io](https://img.shields.io/crates/v/warp.svg)](https://crates.io/crates/warp-contract)
[![workflow](https://github.com/xpladev/warp-contract/actions/workflows/Basic.yml/badge.svg)](https://github.com/xpladev/warp-contract/actions/workflows/Basic.yml)
[![codecov](https://codecov.io/gh/xpladev/warp-contract/branch/main/graph/badge.svg?token=ERMFLEY6Y7)](https://codecov.io/gh/xpladev/warp-contract)


## Contracts

| Name                                               | Description                                  |
| ---------------------------- | -------------------------------------------- |
| [`burner`](contracts/burner) |                                              |
| [`minter`](contracts/minter) |                                              |

* burner on terra-classic

   burner: `terra1`

* minter on xpla

   Mainnet: `xpla`

   Testnet: `xpla`


## Running this contract

You will need Rust 1.46.1+ with wasm32-unknown-unknown target installed.

You can run unit tests on this on each contracts directory via :

```
cargo unit-test
cargo integration-test
```

Once you are happy with the content, you can compile it to wasm on each contracts directory via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw1_subkeys.wasm .
ls -l cw1_subkeys.wasm
sha256sum cw1_subkeys.wasm
```

Or for a production-ready (compressed) build, run the following from the repository root:

```
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.12.6
```

The optimized contracts are generated in the artifacts/ directory.
