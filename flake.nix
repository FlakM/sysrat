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
            pkg-config
            llvmPackages.clang
            cmake
            bpf-linker
            #rust-analyzer-unwrapped
          ];
          buildInputs = with pkgs; [
            llvmPackages.clang
            #rust-toolchain
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];

          env = {
            RUST_SRC_PATH = "${rust-toolchain}/lib/rustlib/src/rust/library";
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          };

          packages = with pkgs; [
            delta
            git
            bpftools
          ]; # Additional dev shell packages can be appended here.
        };
      }
    );
}
