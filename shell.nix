{ rust-overlay ? (import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz")), pkgs ? (import <nixpkgs> {
  crossSystem = {
    config = "x86_64-w64-mingw32";
  };
  overlays = [ rust-overlay ];
})}:

pkgs.callPackage (
{ mkShell, stdenv, rust-bin, windows, wine64 }:
mkShell {
  nativeBuildInputs = [
    rust-bin.stable.latest.minimal
  ];

  depsBuildBuild = [ wine64 ];
  buildInputs = [ windows.pthreads ];

  CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = "${stdenv.cc.targetPrefix}cc";
  CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER = "wine64";
  CARGO_BUILD_TARGET="x86_64-pc-windows-gnu";
  CXXFLAGS_x86_64_pc_windows_gnu="-shared -fno-threadsafe-statics";
}) {}

