{
  description = "KidneyOS";

  inputs = {
    nixpkgs.url = "nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "utils";
      };
    };
  };

  outputs = { nixpkgs, utils, rust-overlay, ... }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          overlays = [ (import rust-overlay) ];
          inherit system;
        };
        inherit (pkgs) bochs gdb grub2 mkShell qemu rust-analyzer rust-bin
          unixtools xorriso;
        inherit (unixtools) xxd;
        rust = rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;
        i686-cc = (import nixpkgs {
          crossSystem = "i686-linux";
          inherit system;
        }).stdenv.cc;
      in
      {
        devShells.default = mkShell {
          packages = [
            bochs
            gdb
            grub2
            i686-cc
            qemu
            rust
            rust-analyzer
            xorriso
            xxd
          ];

          CARGO_TARGET_I686_UNKNOWN_LINUX_GNU_LINKER = "${i686-cc.targetPrefix}cc";
          CARGO_TARGET_I686_UNKNOWN_LINUX_GNU_RUNNER = "${qemu}/bin/qemu-i386";
        };
      });
}
