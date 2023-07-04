# Zellij-supabar
A status bar for zellij

## Features
- Session name
- Current mode
- Tab names, sync, fullscreen and floating status
- Layout name
- Git branch
- Clock

## Install
Obtain the `zellij-supabar.wasm` file via the releases page or by building from source.
There is a nix flake that can help to build the project.
After that load like any other plugin either:
```kdl
pane size=1 borderless=true {
  plugin location="file:path-to-file/zellij-supabar.wasm"
}

```
in your layout or 

```fish
zellij action start-or-reload-plugin file:path-to-file/zellij-supabar.wasm
```

## Building
Youll need a rust toolchain with the wasm32-wasi target enabled.
This can be done with nix via
```fish
nix develop
```
Or by using rustup
```fish
rustup default stable
rustup target add wasm32-wasi
```

Then run 
```
cargo build --release
```
and the file you want should be created under `./target/wasm32-wasi/release/zellij-supabar.wasm`
