{
  description = "Status bar for zellij";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, naersk }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ rust-overlay.overlays.default ];
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        naersk_wasm = pkgs.callPackage naersk { rustc = rust; };
        pkgs = import nixpkgs { inherit system overlays; };

      in rec {
        packages.default = naersk_wasm.buildPackage {
          name = "zellij-supabar";
          src = ./.;
          copyLibs = true;
          CARGO_BUILD_TARGET = "wasm32-wasi";
        };
        overlays.default = (self: super: { supabar = packages.default; });
        apps.default = packages.default;
        devShell = pkgs.mkShell {
          packages = with pkgs; [ watchexec rust-analyzer wasm-bindgen-cli rust ];
        };
      });
}
