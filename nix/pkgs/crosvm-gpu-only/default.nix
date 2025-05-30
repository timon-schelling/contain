pkgs: pkgs.crosvm.overrideAttrs (oldAttrs: {
  patches = [
    ./fix-incorrect-type-on-write-pipes.patch
    ./disable-fbdev-support.patch
  ] ++ oldAttrs.patches;

    buildNoDefaultFeatures = true;
    buildFeatures = [
      "gpu"
    ];

    doCheck = false;
})
