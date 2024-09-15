{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in with pkgs; rec {
        devShell = mkShell rec {
          packages = [
            pkgs.speechd
          ];
          buildInputs = [
            pkgs.libclang
            pkgs.speechd
          ];
          nativeBuildInputs = [
          ];
          LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
        };
      });
}