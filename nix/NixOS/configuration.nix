{ lib, pkgs, modulesPath, system, cs2kz-api, ... }:

let
  sshKeys = [
    ''ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIB4SBKTQ7WJcihtw3QocLXi+xEc/6HklXigYoltI8iNH alphakeks@dawn''
  ];
in

{
  imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];
  boot.initrd = {
    availableKernelModules = [ "ata_piix" "uhci_hcd" "xen_blkfront" ];
    kernelModules = [ "nvme" ];
  };
  environment = {
    systemPackages = with pkgs; [ coreutils vim ];
    defaultPackages = with pkgs; [ tmux curl git btop fd fzf neovim jq ripgrep ];
    variables.EDITOR = "nvim";
  };
  networking = {
    hostName = "cs2kz-api";
    firewall = {
      interfaces = {
        "enp0s6" = {
          allowedTCPPorts = [ 22 80 443 ];
        };
      };
    };
  };
  nixpkgs.hostPlatform = system;
  programs.zsh.enable = true;
  security.acme = {
    acceptTerms = true;
    defaults.email = "cs2kz@dawn.sh";
  };
  services = {
    openssh = {
      enable = true;
      settings.PasswordAuthentication = false;
    };
    mysql = {
      enable = true;
      package = pkgs.mariadb;
      ensureDatabases = [ "cs2kz" ];
      ensureUsers = [{
        name = "schnose";
        ensurePermissions = {
          "cs2kz.*" = "ALL PRIVILEGES"; # TODO: more granular permissions
        };
      }];
      initialDatabases = [{
        name = "cs2kz";
        schema = ../../crates/cs2kz/migrations/0001_initial.up.sql;
      }];
    };
    mysqlBackup = {
      enable = true;
      calendar = "02:30:00";
      databases = [ "cs2kz" ];
    };
    nginx = {
      enable = true;
      recommendedTlsSettings = true;
      recommendedProxySettings = true;
      virtualHosts."api.cs2kz.org" = {
        forceSSL = true;
        enableACME = true;
        locations."/" = {
          proxyPass = "http://[::1]:42069";
          proxyWebsockets = true;
        };
      };
    };
  };
  system.stateVersion = "24.05";
  systemd.services.cs2kz-api = {
    environment = {
      KZ_API_ENVIRONMENT = "production";
    };
    script = ''
      ${cs2kz-api}/bin/cs2kz-api \
        --config "/etc/cs2kz-api.toml" \
        --depot-downloader-path "${pkgs.depotdownloader}/bin/DepotDownloader"
    '';
  };
  time.timeZone = "Europe/Berlin";
  users = {
    defaultUserShell = pkgs.zsh;
    users.root.openssh.authorizedKeys.keys = sshKeys;
    users.schnose = {
      isNormalUser = true;
      useDefaultShell = true;
      extraGroups = [ "wheel" ];
      openssh.authorizedKeys.keys = sshKeys;
    };
  };
}
