{
  description = "Minimal flake providing basic shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    nix-filter.url = "github:numtide/nix-filter";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, nix-filter, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        rust-toolchain = ((pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override
          {
            extensions = [ "rust-src" ];
          }
        );
        overlays = [
          (import rust-overlay)
          (final: prev: {
            nix-filter = nix-filter.lib;
            rust-toolchain = rust-toolchain;
          })
        ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

      in
      {
        devShells.default = pkgs.mkShell {

          nativeBuildInputs = with pkgs; [
            zlib
            pkg-config
            cmake
            rust-analyzer-unwrapped
            protobuf
            xmlsec.dev
          ];
          buildInputs = with pkgs; [
            libxml2
            libxslt
            openssl
            llvmPackages.libclang
            libtool
            xmlsec
            zlib
            rust-toolchain
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];

          env = {
            RUST_SRC_PATH = "${rust-toolchain}/lib/rustlib/src/rust/library";
          };

          packages = with pkgs; [
            delta
            git
          ]; # Additional dev shell packages can be appended here.
        };
      }
    );
}
