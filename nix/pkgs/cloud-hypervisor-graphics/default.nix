pkgs:

let
  patchesSrc = fetchTarball {
    url = "https://spectrum-os.org/software/cloud-hypervisor/cloud-hypervisor-50.0-spectrum0-patches.tar.gz";
    sha256 = "sha256:042g5607kv32bkmyc012i7mhywdmz14na5py41vc7ggd52902q20";
  };
in
pkgs.cloud-hypervisor.overrideAttrs (oldAttrs: rec {
  cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
    inherit patches;
    inherit (oldAttrs) src;
    hash = "sha256-wGtsyKDg1z1QK9mJ1Q43NSjoPbm3m81p++DoD8ipIUI=";
  };

  vhost = pkgs.fetchFromGitHub {
    name = "vhost";
    owner = "rust-vmm";
    repo = "vhost";
    rev = "vhost-user-backend-v0.20.0";
    hash = "sha256-KK1+mwYQr7YkyGT9+51v7TJael9D0lle2JXfRoTqYq8=";
  };

  patches = oldAttrs.patches or [] ++ [
    "${patchesSrc}/cloud-hypervisor.patch"
  ];

  postUnpack = oldAttrs.postUnpack or "" + ''
    unpackFile $vhost
    chmod -R +w vhost
  '';

  postPatch = oldAttrs.postPatch or "" + ''
    pushd ../vhost
    patch -p1 < ${patchesSrc}/vhost.patch
    popd
  '';
})
