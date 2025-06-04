self: { config, lib, pkgs, ... }:

let
  cfg = config.contain;
  json = pkgs.formats.json {};
  defaultCfg = {
    kernel_path = "${config.system.build.kernel}/bzImage";
    initrd_path = "${config.system.build.initialRamdisk}/${config.system.boot.loader.initrdFile}";
    cmdline = "console=hvc0 loglevel=4 reboot=t panic=-1 lsm=on root=fstab init=${config.system.build.toplevel}/init regInfo=${pkgs.closureInfo {rootPaths = [ config.system.build.toplevel ];}}/registration";
    filesystem = {
      shares = [
        {
          tag = "nix-store-ro";
          source = "/nix/store";
          write = false;
        }
      ];
    };
    network = {
      assign_tap_device = true;
    };
    graphics = {
      virtio_gpu = true;
    };
    console = {
      mode = "on";
    };
  };
in
{
  options.contain = {
    enable = lib.mkEnableOption "contain guest";

    config = lib.mkOption {
      type = json.type;
      default = { };
    };

    debug = lib.mkEnableOption "debugging mode";

    optimizations = lib.mkEnableOption "optimizations";

    out = lib.mkOption (
      let
        recursiveMergeImpl = with lib; args:
          zipAttrsWith (n: values:
            if tail values == []
              then head values
            else if all isList values
              then unique (concatLists values)
            else if all isAttrs values
              then recursiveMergeImpl (args ++ [n]) values
            else last values
          );
        recursiveMerge = args: recursiveMergeImpl [] args;
      in
      {
        type = lib.types.path;
        default = json.generate "contain-config.json" (recursiveMerge [defaultCfg cfg.config]);
      }
    );

    bin = lib.mkOption {
      type = lib.types.package;
      default = pkgs.writeScriptBin "contain-start" ''
        #!${lib.getExe pkgs.bash}
        exec ${lib.getExe self.packages.${pkgs.system}.contain} start "${cfg.out}"
      '';
    };
  };

  config = lib.mkMerge [
    (lib.mkIf (cfg.enable) {
      boot = {
        initrd.kernelModules = [
          "virtio_pci"
          "virtio_blk"
          "virtio_console"
          "virtiofs"
          "overlay"
        ];
        kernelModules = [
          "drm"
          "virtio_gpu"
        ];
        blacklistedKernelModules = [
          "rfkill"
          "intel_pstate"
        ];
      };

      boot.initrd.systemd.enable = true;
      boot.initrd.systemd.tpm2.enable = false;
      boot.loader.grub.enable = false;
      systemd.tpm2.enable = false;

      fileSystems = {
        "/" = {
          device = "rootfs";
          fsType = "tmpfs";
          mountPoint = "/";
          neededForBoot = true;
          options = [
            "x-initrd.mount"
            "size=50%,mode=0755"
          ];
        };
        "/nix/.ro-store" = {
          device = "nix-store-ro";
          fsType = "virtiofs";
          mountPoint = "/nix/.ro-store";
          neededForBoot = true;
          options = [
            "x-initrd.mount"
            "defaults"
            "x-systemd.requires=systemd-modules-load.service"
          ];
        };
        "/nix/store" = {
          depends = [
            "/nix/.ro-store"
            "/nix/.rw-store"
          ];
          device = "overlay";
          fsType = "overlay";
          mountPoint = "/nix/store";
          neededForBoot = true;
          options = [
            "x-initrd.mount"
            "x-systemd.requires-mounts-for=/sysroot/nix/.ro-store"
            "x-systemd.requires-mounts-for=/sysroot/nix/.rw-store"
            "lowerdir=/nix/.ro-store"
            "upperdir=/nix/.rw-store/store"
            "workdir=/nix/.rw-store/work"
          ];
        };
      };

      boot.initrd.systemd.mounts = [
        {
          where = "/sysroot/nix/store";
          what = "overlay";
          type = "overlay";
          options = builtins.concatStringsSep "," [
            "lowerdir=/sysroot/nix/.ro-store"
            "upperdir=/sysroot/nix/.rw-store/store"
            "workdir=/sysroot/nix/.rw-store/work"
          ];
          wantedBy = [ "initrd-fs.target" ];
          before = [ "initrd-fs.target" ];
          requires = [ "rw-store.service" ];
          after = [ "rw-store.service" ];
          unitConfig.RequiresMountsFor = "/sysroot/nix/.ro-store";
        }
      ];

      boot.initrd.systemd.services.rw-store = {
        unitConfig = {
          DefaultDependencies = false;
          RequiresMountsFor = "/sysroot/nix/.rw-store";
        };
        serviceConfig = {
          Type = "oneshot";
          ExecStart = ''
            /bin/mkdir -p -m 0755 \
              /sysroot/nix/.rw-store/store \
              /sysroot/nix/.rw-store/work \
              /sysroot/nix/store
          '';
        };
      };

      systemd.sockets.nix-daemon.enable = lib.mkDefault true;
      systemd.services.nix-daemon.enable = lib.mkDefault true;
      boot.postBootCommands = ''
        if [[ "$(cat /proc/cmdline)" =~ regInfo=([^ ]*) ]]; then
          ${config.nix.package.out}/bin/nix-store --load-db < ''${BASH_REMATCH[1]}
        fi
      '';

      systemd.user.services.wayland-proxy = {
        enable = true;
        description = "wayland proxy";
        serviceConfig = {
          ExecStart = "${pkgs.wayland-proxy-virtwl}/bin/wayland-proxy-virtwl --virtio-gpu --tag='[vm] - ' --x-display=0 --xwayland-binary=${pkgs.xwayland}/bin/Xwayland";
          Restart = "on-failure";
          RestartSec = 5;
        };
        wantedBy = [ "default.target" ];
      };

      environment.sessionVariables = {
        WAYLAND_DISPLAY = "wayland-1";
        DISPLAY = ":0";
        XDG_SESSION_TYPE = "wayland";
      };
    })
    (lib.mkIf (cfg.enable && cfg.debug) {
      boot.initrd.systemd.emergencyAccess = true;
    })
    (lib.mkIf (cfg.enable && cfg.optimizations && !cfg.debug) {
      documentation.enable = false;
      console.earlySetup = false;
      services.logrotate.enable = false;
      networking.firewall.enable = false;
      systemd.network.wait-online.enable = false;
      systemd.services.mount-pstore.enable = false;
      systemd.services.logrotate-checkconf.enable = false;
      systemd.services.systemd-journal-flush.enable = false;
      systemd.services.kmod-static-nodes.enable = false;
      systemd.services.systemd-journal-catalog-update.enable = false;
      systemd.services.systemd-logind.serviceConfig = {
        StandardOutput = "null";
        StandardError = "null";
      };
      systemd.sockets.systemd-journald-audit.enable = false;
      systemd.sockets.systemd-journald-dev-log.enable = false;

      boot.initrd.systemd.services.systemd-vconsole-setup.enable = false;
      boot.initrd.systemd.services.systemd-journald.enable = false;
      boot.initrd.systemd.services.systemd-journal-flush.enable = false;
      boot.initrd.systemd.sockets.systemd-journald-audit.enable = false;
    })
  ];
}
