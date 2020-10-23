# CW20 Clawback

This is a prototype contract code for "clawbacks" of native and CW20 tokens. A clawback works as follows:
- There is a "holder" key/account, a "backup" key/account, and a "clawback period" (which determines when the clawback expires).
- Within a "clawback period", "holder" can transfer to "holders" / other clawbacks (provided their terms match the outgoing contract: they have the same "backup", "clawback period" is at least as long, and they support the same tokens) or refresh the clawback duration. After the clawback period expires, "holder" can withdraw the tokens.
- Within a "clawback period", "backup" can transfer to other holder, refresh the clawback duration or burn the tokens / destroy the contract.

There are at least two potential use cases of this logic:
1. Exchange hot/cold wallet management protocols (similar to [Bitcoin Vaults](https://arxiv.org/abs/2005.11776) with covenants): the "backup" key here is used for retrieving back (or destroying if the "backup" key leaked too) stolen funds.
2. Cashbacks: the "backup" key here roughly corresponds to the issuer / payment processor which serves merchants (that are paid in fiat) and gives cashbacks to customers -- if customer order are cancelled, modified or goods are returned, full or partial cashbacks are taken back.

## Running this contract

You will need Rust 1.44.1+ with `wasm32-unknown-unknown` target installed.

You can run unit tests on this via: 

`cargo test`

Once you are happy with the content, you can compile it to wasm via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw20_clawback.wasm .
ls -l cw20_clawback.wasm
sha256sum cw20_clawback.wasm
```

Or for a production-ready (optimized) build, run a build command, as suggested in CosmWasm:

```
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.9.0
```

## HackAtom Testnet
The optimized binary was uploaded to the HackAtom testnet (running on 0.11.1 CosmWasm)
and got the id `28`,
so you can just use it without re-uploading it:

```
wasmcli tx wasm instantiate 28 "{}" --from <YOUR KEY> --label "<SOME LABEL>" --gas 100000 -y

wasmcli tx wasm execute <INSTANTIATED-CONTRACT-ADDRESS> '{"create": {"id": "<ID>", "backup": "<ADDR1>", "holder": "<ADDR2>", "clawback_period": <TIME>}}' --from <YOUR KEY> --gas 100000 --amount=<SOME AMOUNT>ucosm -y

wasmcli tx wasm execute <INSTANTIATED-CONTRACT-ADDRESS> '{"withdraw": {"id": "<ID>"}}' --from <KEY FOR ADDR2> --gas 100000 -y

...

```