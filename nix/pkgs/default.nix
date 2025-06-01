{ pkgs, ... }:

rec {
  contain-unwrapped = pkgs.rustPlatform.buildRustPackage {
    pname = "contain-unwrapped";
    version = "0.1.0";
    src = ../..;
    useFetchCargoVendor = true;
    cargoHash = "sha256-yXVA5f6v2EOL15RIZlGnQgtTmLUSs6p4OARUlYXaZtE=";
  };
  contain = (pkgs.runCommand "contain" {
    buildInputs = [ pkgs.makeWrapper ];
    meta.mainProgram = "contain";
  } ''
    makeWrapper ${contain-unwrapped}/bin/contain $out/bin/contain \
      --set PATH ${pkgs.lib.makeBinPath ([ cloud-hypervisor-graphics virtiofsd crosvm-gpu-only ])}
  '');
  containd = (pkgs.runCommand "containd" {
    buildInputs = [ pkgs.makeWrapper ];
    meta.mainProgram = "containd";
  } ''
    makeWrapper ${contain-unwrapped}/bin/containd $out/bin/containd \
      --set PATH ${pkgs.lib.makeBinPath (with pkgs; [ iproute2 ])}
  '');

  cloud-hypervisor-graphics = import ./cloud-hypervisor-graphics pkgs;
  virtiofsd = import ./virtiofsd pkgs;
  crosvm-gpu-only = import ./crosvm-gpu-only pkgs;
}
