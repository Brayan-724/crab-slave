{
  inputs.nixpkgs.url = "github:nixos/nixpkgs";

  outputs = {nixpkgs, ...}: let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
    libraries = with pkgs; [
      ffmpeg_6
      ffmpeg_6.dev
      pkg-config
      clang
      libclang
    ];
  in {
    devShells.${system}.default = pkgs.mkShell {
      buildInputs = libraries;

      buildInputsNative = [pkgs.pkg-config];

      LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath libraries;
    };
  };
}
