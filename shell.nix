{ fenix ? (import (builtins.fetchTarball "https://github.com/nix-community/fenix/archive/main.tar.gz") { })
, pkgs ? (import <nixpkgs> {
    crossSystem = {
      config = "x86_64-w64-mingw32";
    };
  })
, system ? builtins.currentSystem
}:
let
  fenix' = fenix.packages.${system};
in
pkgs.callPackage
  ({ mkShell, lib, buildPackages, stdenv, windows, libgit2, pkg-config }: mkShell rec {
    buildInputs = [
      stdenv.cc
      windows.pthreads
  ];

  depsBuildBuild = [
      pkg-config
  ];

  LD_LIBRARY_PATH="${lib.makeLibraryPath [buildPackages.buildPackages.libgit2]}";
    nativeBuildInputs = [
      buildPackages.stdenv.cc
      libgit2
      (fenix'.combine [
        (fenix'.complete.withComponents [
          "cargo"
          "rust-src"
          "clippy"
          "rustc"
        ])
        fenix'.rust-analyzer
        fenix'.latest.rustfmt
        fenix'.targets.x86_64-pc-windows-gnu.latest.rust-std
      ])
    ];

    LIBGIT2_NO_VENDOR=1;
    CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";
    TARGET_CC = "${stdenv.cc.targetPrefix}cc";
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = TARGET_CC;
    CXXFLAGS_x86_64_pc_windows_gnu = "-Oz -shared -fno-threadsafe-statics";
  })
{ }
