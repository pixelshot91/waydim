{
  description = "waydim - hybrid hardware + software display dimmer (Wayland compatible)";
  
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    rust-overlay.inputs.flake-utils.follows = "flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "waydim";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          
          buildInputs = with pkgs; [
            pkg-config
          ];
          
          nativeBuildInputs = with pkgs; [
            rustToolchain
          ];
          
          meta = with pkgs.lib; {
            description = "Hybrid hardware + software display dimmer (Wayland compatible)";
            license = licenses.mit;
            maintainers = [];
          };
        };

        devShells.default = pkgs.mkShell {
          name = "waydim-dev";
          buildInputs = with pkgs; [
            rustToolchain
            cargo
            rustfmt
            clippy
            pkg-config
          ];
        };
      }
    );
}
