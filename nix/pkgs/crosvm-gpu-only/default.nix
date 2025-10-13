pkgs: pkgs.crosvm.overrideAttrs (oldAttrs: {
  patches = [
    ./disable-fbdev-support.patch
  ] ++ oldAttrs.patches;

    buildNoDefaultFeatures = true;
    buildFeatures = [
      "gpu"
    ];

    doCheck = false;
})
