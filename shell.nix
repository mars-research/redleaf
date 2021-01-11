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
  pinnedRust = rustChannel.rust.override {
    extensions = [ "rust-src" ];
  };
in pkgs.mkShell {
  buildInputs = with pkgs; [
    pinnedRust

    gnumake utillinux

    gcc10 clang_10 nasm
    qemu grub2 xorriso
  ];
}
