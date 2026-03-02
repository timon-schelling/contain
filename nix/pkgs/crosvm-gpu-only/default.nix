pkgs: (pkgs.callPackage ./package.nix {}).overrideAttrs (oldAttrs: {
  patches = [
    ./disable-fbdev-support.patch
  ] ++ oldAttrs.patches;

  buildNoDefaultFeatures = true;
  buildFeatures = [
    "gpu"
  ];

  doCheck = false;
})
