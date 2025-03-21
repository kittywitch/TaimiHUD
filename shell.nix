{
  fenix ? (import (builtins.fetchTarball "https://github.com/nix-community/fenix/archive/main.tar.gz") { }),
  pkgs ? (import <nixpkgs> {
  crossSystem = {
    config = "x86_64-w64-mingw32";
  };
  }),
  system ? builtins.currentSystem
}: let
  fenix' = fenix.packages.${system};
  pkgsCross = pkgs.pkgsCross.mingwW64;
in pkgs.callPackage({ mkShell, buildPackages, stdenv, windows}: mkShell rec {
  depsBuildBuild = [
      pkgsCross.stdenv.cc
      pkgsCross.windows.pthreads
  ];

  nativeBuildInputs = [
    buildPackages.stdenv.cc
    (fenix'.combine [
        (fenix'.complete.withComponents [
        "cargo"
        "clippy"
        "rust-src"
        "rustc"
      ])
      fenix'.rust-analyzer
      fenix'.latest.rustfmt
      fenix'.targets.x86_64-pc-windows-gnu.latest.rust-std
      ])
    ];

  CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";
  TARGET_CC = "${pkgsCross.stdenv.cc.targetPrefix}cc";
  CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = TARGET_CC;
  CXXFLAGS_x86_64_pc_windows_gnu="-shared -fno-threadsafe-statics";
}) {}
