{
  sources ? import ./nix/sources.nix,
  rustNightly ? "2021-01-10",
  pkgs ? import sources.nixpkgs {
    overlays = [
      (import sources.nixpkgs-mozilla)
    ];
  },
}: let
  rustChannel = pkgs.rustChannelOf {
    channel = "nightly";
    date = rustNightly;
  };
  rustPlatform = pkgs.makeRustPlatform {
    rustc = pinnedRust;
    cargo = pinnedRust;
  };
  pinnedRust = rustChannel.rust.override {
    extensions = [ "rust-src" ];
  };
  cargoExpand = pkgs.cargo-expand.override {
    inherit rustPlatform;
  };
in pkgs.mkShell {
  buildInputs = with pkgs; [
    pinnedRust cargoExpand

    gnumake utillinux

    gcc10 clang_10 nasm
    qemu grub2 xorriso
  ];
  shellHook = ''
    export SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
  '';
}
