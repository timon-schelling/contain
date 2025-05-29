{ nixpkgs, ... }:

let
  forAllSystems = function:
    nixpkgs.lib.genAttrs [
      "x86_64-linux"
      "aarch64-linux"
    ] (system: function nixpkgs.legacyPackages.${system});
in {
  packages = forAllSystems (pkgs: import ./pkgs pkgs);
}
