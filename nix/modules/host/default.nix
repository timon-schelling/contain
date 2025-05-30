self: { config, lib, pkgs, ... }:

let
  cfg = config.contain.host;
in
{
  options.contain.host = {
    enable = lib.mkEnableOption "contain host module";
    network = {
      enable = lib.mkEnableOption "contain host network configuration";
      bridgeName = lib.mkOption {
        type = lib.types.str;
        default = "contain-bridge";
        description = "Name of the bridge interface that contain vms will connect to";
      };
    };
  };

  config = lib.mkMerge [
    (lib.mkIf (cfg.enable) {
      environment.systemPackages = [
        self.packages.${pkgs.system}.contain
      ];
      systemd.services."containd" = {
        enable = true;
        description = "contain daemon";
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${lib.getExe self.packages.${pkgs.system}.containd}";
          Restart = "always";
        };
      };
    })
    (lib.mkIf (cfg.enable && cfg.network.enable) {
      systemd.network = {
        networks."11-contain-vm-all" = {
          matchConfig.Name = "contain-*";
          networkConfig.Bridge = cfg.network.bridgeName;
        };
      };
    })
  ];
}
