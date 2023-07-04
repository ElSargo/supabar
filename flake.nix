{
  description = "Minimal rust wasm32-unknown-unknown example";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs { inherit system overlays; };
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        inputs = with pkgs;
          [ wasm-bindgen-cli glibc gcc cmake glibc stdenv.cc ] ++ [ rust ];
        native_inputs = with pkgs; [ gcc cmake glibc stdenv.cc ];

      in {

        packages = with pkgs; {
          default = rustPlatform.buildRustPackage {
            name = "zellij-supabar";
            src = ./.;
            buildInputs = inputs;
            nativeBuildInputs = native_inputs;
            target = "wasm32-wasi";
            cargoSha256 = "sha256-X45FXTqTFk0ysenN5EoIefz5qY0qfTKYveF/11+1OtY=";

          };
          LD_LIBRARY_PATH = nixpkgs.lib.makeLibraryPath inputs;
        };

        devShell = pkgs.mkShell {
          packages = inputs ++ [ pkgs.watchexec pkgs.rust-analyzer ];
          LD_LIBRARY_PATH = nixpkgs.lib.makeLibraryPath inputs;

        };

      });
}
