pkgs: pkgs.virtiofsd.overrideAttrs (oldAttrs: {
  version = "unstable-2025-06-01";
  src = pkgs.fetchFromGitLab {
    owner = "virtio-fs";
    repo = "virtiofsd";
    rev = "1e1c5dee74258fff83bd550f8df340798103eeba";
    hash = "sha256-8chYTBMpNiVmb1opbW5PIr5VCehsFyBL635p2ys1z80";
  };
})
