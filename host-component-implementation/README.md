# Using this

`kv-custom.wit` contains a world that uses one interface of the KV world, crud.

## Building

```sh
pushd kv-custom
cargo component build --release
popd
pushd kv-custom-host
```

## Running

Run host component passing in the name of the key you want to set the value "foo" in.

```sh
cargo run -- "mykey" ../kv-custom/target/wasm32-wasi/release/kv_custom.wasm
```