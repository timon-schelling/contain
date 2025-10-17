pkgs: pkgs.crosvm.overrideAttrs (oldAttrs: {
  patches = [
    ./disable-fbdev-support.patch
  ] ++ oldAttrs.patches;

  postPatch = oldAttrs.postPatch or "" + ''
    # see https://github.com/magma-gpu/rutabaga_gfx/issues/18
    substituteInPlace $cargoDepsCopy/rutabaga_gfx-*/src/cross_domain/mod.rs \
        --replace-fail \
        "Ok(DescriptorType::WritePipe) => {
                                        *identifier_type = CROSS_DOMAIN_ID_TYPE_VIRTGPU_BLOB;" \
        "Ok(DescriptorType::WritePipe) => {
                                        *identifier_type = CROSS_DOMAIN_ID_TYPE_WRITE_PIPE;"
  '';

  buildNoDefaultFeatures = true;
  buildFeatures = [
    "gpu"
  ];

  doCheck = false;
})
