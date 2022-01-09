{
  description = "RedLeaf Operating System";

  inputs = {
    mars-std.url = "github:mars-research/mars-std";
  };

  outputs = { self, mars-std, ... }: let
    supportedSystems = [ "x86_64-linux" ];
  in mars-std.lib.eachSystem supportedSystems (system: let
    nightlyVersion = "2021-12-15";

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
      SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";

      nativeBuildInputs = [
        pinnedRust cargoExpand
      ] ++ (with pkgs; [
        gnumake utillinux which

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
