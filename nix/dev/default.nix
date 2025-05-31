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
        };
      };

      environment.systemPackages = [
        pkgs.cargo
        pkgs.rustc
        pkgs.gcc
        pkgs.lldb
        (pkgs.vscode-with-extensions.override {
          vscodeExtensions = with pkgs.vscode-extensions;[
            rust-lang.rust-analyzer
            vadimcn.vscode-lldb
            tamasfe.even-better-toml
          ];
        })
      ];

      nixpkgs.config.allowUnfree = true;

      nixpkgs.hostPlatform = "x86_64-linux";
    })
  ];
}
