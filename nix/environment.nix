let
  sources = import ./sources.nix;
  rustNightly = "2021-01-10";
  pkgs = import sources.nixpkgs {
    overlays = [
      (import sources.nixpkgs-mozilla)
    ];
  };

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
in {
  inherit pkgs;

  dependencies = with pkgs; [
    pinnedRust cargoExpand

    gnumake utillinux

    gcc10 clang_10 nasm
    qemu grub2 xorriso
  ];

  # Not necessary for building redleaf itself, but useful for a dev shell
  shellDependencies = with pkgs; [
    coreutils bash which git vim tmux
    gdb
  ];
}
