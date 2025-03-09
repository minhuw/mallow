{
  description = "mallow - A Rust project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            pre-commit
          ];

          shellHook = ''
            export RUST_SRC_PATH=${rustToolchain}/lib/rustlib/src/rust/library
          '';
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "mallow";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            rustToolchain
          ];

          buildInputs = with pkgs; [
            openssl
          ];

          RUSTC = "${rustToolchain}/bin/rustc";
          CARGO = "${rustToolchain}/bin/cargo";
        };
      }
    );
} 