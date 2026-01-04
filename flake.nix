{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      pkgs = import nixpkgs { system = "x86_64-linux"; };
    in
    {
      devShells.x86_64-linux.default = pkgs.mkShell {
        inputsFrom = [ pkgs.xorg.libX11 ];
        nativeBuildInputs = with pkgs; [
          rustc
          cargo
          pkgconf
        ];

        buildInputs = with pkgs; [
          alsa-lib
          jack2
          libGL
          libx11
          wayland
          fontconfig
          freetype
        ];
      };
    };
}
