{ pkgs, ... }:

rec {
  contain-unwrapped = pkgs.rustPlatform.buildRustPackage {
    pname = "contain-unwrapped";
    version = "0.1.0";
    src = ../..;
    cargoHash = "sha256-mUrG12+cnrdWnX2rJGGmmt/tjL1GT+g5s2izoKaPOEU=";
  };
  contain = (pkgs.runCommand "contain" {
    buildInputs = [ pkgs.makeWrapper ];
    meta.mainProgram = "contain";
  } ''
    makeWrapper ${contain-unwrapped}/bin/contain $out/bin/contain \
      --set PATH ${pkgs.lib.makeBinPath [ cloud-hypervisor-graphics crosvm-gpu-only pkgs.virtiofsd ]}
  '');
  containd = (pkgs.runCommand "containd" {
    buildInputs = [ pkgs.makeWrapper ];
    meta.mainProgram = "containd";
  } ''
    makeWrapper ${contain-unwrapped}/bin/containd $out/bin/containd \
      --set PATH ${pkgs.lib.makeBinPath [ pkgs.iproute2 ]}
  '');

  cloud-hypervisor-graphics = import ./cloud-hypervisor-graphics pkgs;
  crosvm-gpu-only = import ./crosvm-gpu-only pkgs;
}
