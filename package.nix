{ craneLib, pkgs }:

let
    pkgsCross = pkgs.pkgsCross.mingwW64;
in craneLib.buildPackage rec {
  src = ./.;
  strictDeps = true;

  depsBuildBuild = with pkgsCross; [
    stdenv.cc
    windows.pthreads
    libgit2
  ];

  nativeBuildInputs = [
    pkg-config
  ];

  doCheck = false;

  # Tells Cargo that we're building for Windows.
  # (https://doc.rust-lang.org/cargo/reference/config.html#buildtarget)
  CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";

  TARGET_CC = "${pkgsCross.stdenv.cc}/bin/${pkgsCross.stdenv.cc.targetPrefix}cc";

  # Build without a dependency not provided by wine
  CXXFLAGS_x86_64_pc_windows_gnu = "-shared -fno-threadsafe-statics";

  CARGO_BUILD_RUSTFLAGS = [
    "-C"
    "linker=${TARGET_CC}"
  ];
}
