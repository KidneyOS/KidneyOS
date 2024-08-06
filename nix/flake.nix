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

  outputs = { self, nixpkgs, utils, rust-overlay }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          overlays = [ (import rust-overlay) ];
          inherit system;
        };
        inherit (pkgs) bochs gdb gnumake grcov mdbook mdsh mkShell mtools qemu
          rust-bin shellcheck unixtools xorriso;
        inherit (unixtools) xxd;
        rust = rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;
        i686-pkgs = import nixpkgs {
          crossSystem = "i686-linux";
          inherit system;
        };
        i686-cc = i686-pkgs.stdenv.cc;
        grub2 =
          if system == "x86_64-linux"
          then pkgs.grub2
          else
            pkgs.runCommand "grub2-mkrescue-i686-cross"
              {
                buildInputs = [ pkgs.makeWrapper ];
              } ''
              makeWrapper ${pkgs.grub2}/bin/grub-mkrescue $out/bin/grub-mkrescue \
                --add-flags "--directory=${i686-pkgs.grub2}/lib/grub/i386-pc"
            '';
      in
      {
        packages.kidneyos-builder = pkgs.dockerTools.buildNixShellImage {
          name = "ghcr.io/kidneyos/kidneyos-builder";
          tag = "latest";
          drv = self.devShells.${system}.build;
        };

        devShells = {
          # If you make changes to this, once they're merged be sure to push
          # updated Docker containers to the GitHub container registry by
          # running:
          #
          # ```sh
          # scripts/build-container.bash && scripts/push-container.bash
          # ```
          #
          # ...on a x86_64-linux host and the same commands but with the `--arm`
          # flags added on a aarch64-linux host.
          build = mkShell {
            packages = [
              gnumake
              grcov
              grub2
              i686-cc
              mdbook
              mtools
              qemu
              rust
              xorriso
            ];

            CARGO_TARGET_I686_UNKNOWN_LINUX_GNU_LINKER = "${i686-cc.targetPrefix}cc";
            CARGO_TARGET_I686_UNKNOWN_LINUX_GNU_RUNNER = "${qemu}/bin/qemu-i386";

            shellHook = ''
              export CARGO_TARGET_DIR="$PWD/build/target"
            '';
          };

          default = self.devShells.${system}.build.overrideAttrs (oldAttrs: {
            nativeBuildInputs = oldAttrs.nativeBuildInputs ++ [
              bochs
              gdb
              mdsh
              shellcheck
              xxd
            ];
          });
        };
      });
}
