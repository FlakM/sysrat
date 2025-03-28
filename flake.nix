{
  description = "Minimal flake providing basic shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    nix-filter.url = "github:numtide/nix-filter";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, flake-utils, nix-filter, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
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
            rustup
            #rust-toolchain
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];

          env = {
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
