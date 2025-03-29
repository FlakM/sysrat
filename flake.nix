{
  description = "Minimal flake providing basic shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    # from master
    nixpkgs-unstable.url = "github:NixOS/nixpkgs/master";
    nix-filter.url = "github:numtide/nix-filter";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, nixpkgs-unstable, flake-utils, nix-filter, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs-old = import nixpkgs {
          inherit system;
        };
        pkgs = import nixpkgs {
          inherit system;
        };
      in
      {
        devShells.default = pkgs.mkShell {

          nativeBuildInputs = with pkgs; [
            pkg-config
            clang
            llvm_19
            cmake
            elfutils
            bpftools
            #bpf-linker
          ];
          buildInputs = with pkgs; [
            clang
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

            #bpf-linker
            bpftools
          ]; # Additional dev shell packages can be appended here.
        };
      }
    );
}
