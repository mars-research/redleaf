{
  description = "RedLeaf Operating System";

  inputs = {
    mars-std.url = "github:mars-research/mars-std";
  };

  outputs = { self, mars-std, ... }: let
    supportedSystems = [ "x86_64-linux" ];
  in mars-std.lib.eachSystem supportedSystems (system: let
    nightlyVersion = "2021-09-07";

    pkgs = mars-std.legacyPackages.${system};
    pinnedRust = pkgs.rust-bin.nightly.${nightlyVersion}.default.override {
      extensions = [ "rust-src" "rust-analyzer-preview" ];
      targets = [ "x86_64-unknown-linux-gnu" ];
    };
    rustPlatform = pkgs.makeRustPlatform {
      rustc = pinnedRust;
      cargo = pinnedRust;
    };
    cargoExpand = pkgs.cargo-expand.override { inherit rustPlatform; };
  in {
    devShell = pkgs.mkShell {
      nativeBuildInputs = [
        pinnedRust cargoExpand
      ] ++ (with pkgs; [
        gnumake utillinux

        gcc10 clang_10 nasm
        qemu grub2 xorriso gdb
        zlib
      ]);
    };

    reproduce = pkgs.mars-research.mkReproduceHook {
      cloudlab = "c220g2";
      script = ''
        echo "OK"
      '';
    };
  });
}
