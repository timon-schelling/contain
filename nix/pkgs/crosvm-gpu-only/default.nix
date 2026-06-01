pkgs: (pkgs.callPackage ./package.nix {}).overrideAttrs (oldAttrs: {
  patches = [
    ./disable-fbdev-support.patch
  ] ++ oldAttrs.patches;

  buildNoDefaultFeatures = true;
  buildFeatures = [
    "gpu"
    "wl-dmabuf"
    "virgl_renderer"
    "virgl_renderer_next"
    "default-no-sandbox"
  ];

  doCheck = false;
})
