pkgs: pkgs.cloud-hypervisor.overrideAttrs (oldAttrs: rec {
  cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
    inherit patches;
    inherit (oldAttrs) src;
    hash = "sha256-IlwX7n12Q3iRcH93oUbF13J0d9oO5OaH4E3MsPibiGo=";
  };

  vhost = pkgs.fetchFromGitHub {
    name = "vhost";
    owner = "rust-vmm";
    repo = "vhost";
    rev = "vhost-user-backend-v0.20.0";
    hash = "sha256-KK1+mwYQr7YkyGT9+51v7TJael9D0lle2JXfRoTqYq8=";
  };

  patches = oldAttrs.patches or [] ++ [
    ./0001-build-use-local-vhost.patch
    ./0002-virtio-devices-add-a-GPU-device.patch
  ];

  vhostPatches = [
    vhost/0001-vhost_user-add-get_size-to-MsgHeader.patch
    vhost/0002-vhost-fix-receiving-reply-payloads.patch
    vhost/0003-vhost_user-add-shared-memory-region-support.patch
    vhost/0004-vhost_user-add-protocol-flag-for-shmem.patch
  ];

  postUnpack = oldAttrs.postUnpack or "" + ''
    unpackFile $vhost
    chmod -R +w vhost
  '';

  postPatch = oldAttrs.postPatch or "" + ''
    pushd ../vhost
    for patch in $vhostPatches; do
        echo applying patch $patch
        patch -p1 < $patch
    done
    popd
  '';
})
