{ lib, self, ... }:

lib.nixosSystem {
  modules = [
    self.nixosModules.guest
    ({ pkgs, ... }: {

      contain = {
        enable = true;
        optimizations = true;
        config = {
          cpu = {
            cores = 16;
          };
          memory = {
            size = 16384;
          };
          filesystem = {
            disks = [
              {
                source = "target/dev.disk.qcow2";
                tag = "target";
                size = 30000;
              }
            ];
            shares = [
              {
                tag = "workspace-rw";
                source = ".";
                write = true;
                inode_file_handles = "never";
              }
            ];
          };
        };
      };

      fileSystems = {
        "/home/user/target" = {
          device = "/dev/disk/by-id/virtio-target";
          fsType = "btrfs";
          neededForBoot = true;
          autoFormat = true;
          options = [
            "x-initrd.mount"
            "defaults"
            "x-systemd.requires=systemd-modules-load.service"
          ];
        };
        "/home/user/workspace" = {
          device = "workspace-rw";
          fsType = "virtiofs";
          mountPoint = "/home/user/workspace";
          options = [
            "defaults"
            "x-systemd.requires=systemd-modules-load.service"
          ];
        };
      };

      systemd.services.own-target-dir = {
        wantedBy = [ "multi-user.target" ];
        after = [ "local-fs.target" ];
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
          ExecStart = ''
            ${pkgs.coreutils}/bin/chown user:users /home/user/target
          '';
        };
      };

      environment.systemPackages = [
        pkgs.cargo
        pkgs.rustc
        pkgs.gcc
        pkgs.lldb
        pkgs.foot
        (pkgs.vscode-with-extensions.override {
          vscodeExtensions = with pkgs.vscode-extensions;[
            rust-lang.rust-analyzer
            vadimcn.vscode-lldb
            tamasfe.even-better-toml
          ];
        })
        (pkgs.writeScriptBin "term" ''
          #!${lib.getExe pkgs.bash}
          exec ${lib.getExe pkgs.foot}
        '')
        (pkgs.writeScriptBin "stop" ''
          #!${lib.getExe pkgs.bash}
          exec sudo poweroff
        '')
      ];

      programs.bash.shellInit = ''
        # cd to /home/user/workspace if in pwd /home/user
        if [ "$PWD" = "/home/user" ]; then
          cd workspace
        fi
      '';

      environment.variables = {
        NIXOS_OZONE_WL = "1";
        GTK_BACKEND = "wayland";
        EGL_PLATFORM = "wayland";
        WLR_BACKENDS = "wayland";
        AQ_BACKEND = "wayland";
        CARGO_TARGET_DIR = "/home/user/target";
      };

      users.users.user = {
        isNormalUser = true;
        extraGroups = [ "users" "wheel" ];
      };

      users.groups.users = { };

      hardware.graphics.enable = true;

      security.sudo.wheelNeedsPassword = false;

      services.getty.autologinUser = "user";

      nixpkgs.config.allowUnfree = true;

      nixpkgs.hostPlatform = "x86_64-linux";
    })
  ];
}
