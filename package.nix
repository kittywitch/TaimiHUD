{ lib, buildPackages, craneLib, stdenv, windows, libgit2, pkg-config, features ? []}:

craneLib.buildPackage rec {
  src = ./.;
  strictDeps = true;
  cargoExtraArgs = if features != [] then lib.escapeShellArgs (["--features"] ++ features) else "";

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
  ];

  doCheck = false;

  LIBGIT2_NO_VENDOR = 1;

  # Tells Cargo that we're building for Windows.
  # (https://doc.rust-lang.org/cargo/reference/config.html#buildtarget)
  CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";

  #TARGET_CC = "${pkgsCross.stdenv.cc}/bin/${pkgsCross.stdenv.cc.targetPrefix}cc";
  TARGET_CC = "${stdenv.cc.targetPrefix}cc";

  # Build without a dependency not provided by wine
  CXXFLAGS_x86_64_pc_windows_gnu = "-Oz -shared -fno-threadsafe-statics";
  PROFILE="release";
  CARGO_BUILD_RUSTFLAGS = [
    "-C"
    "linker=${TARGET_CC}"
  ];
}
