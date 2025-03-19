{
  description = "TaimiHUD; timers, markers and hopefully paths for raidcore.gg nexus";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay/master";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { ... }@inputs: let
    rust-overlay = import inputs.rust-overlay;
    pkgs = (import inputs.nixpkgs {
      system = "x86_64-linux";
      crossSystem = {
        config = "x86_64-w64-mingw32";
      };
      overlays = [ rust-overlay ];
    });
  in {
    devShells."x86_64-linux".default = import ./shell.nix {
      inherit pkgs rust-overlay;
    };
  };
}
