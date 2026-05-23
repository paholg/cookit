{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.cookit;
in
{
  options.services.cookit = {
    enable = lib.mkEnableOption "recipe management service";

    package = lib.mkPackageOption pkgs "cookit" { };

    databaseUrl = lib.mkOption {
      type = lib.types.str;
      description = "Location of the sqlite db";
    };

    cookitUrl = lib.mkOption {
      type = lib.types.str;
      description = "Public URL";
      example = "https://authit.example.com";
    };

    logLevel = lib.mkOption {
      type = lib.types.enum [
        "trace"
        "debug"
        "info"
        "warn"
        "error"
      ];
      default = "info";
      description = "Log level for the service";
    };

    ipAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "Bind address";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8080;
      description = "Port to listen on";
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Open firewall port";
    };

    user = lib.mkOption {
      type = lib.types.str;
      default = "cookit";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = "cookit";
    };
  };

  config = lib.mkIf cfg.enable {
    users.users.${cfg.user} = {
      isSystemUser = true;
      group = cfg.group;
      home = "/var/lib/cookit";
    };

    users.groups.${cfg.group} = { };

    systemd.services.authit = {
      description = "CookIt recipes";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      environment = {
        DATABASE_URL = cfg.databaseUrl;
        IP = cfg.ipAddress;
        PORT = toString cfg.port;
        URL = cfg.authitUrl;
        LOG_LEVEL = cfg.logLevel;
      };

      serviceConfig = {
        ExecStart = "${cfg.package}/bin/web";
        StateDirectory = "authit";
        User = cfg.user;
        Group = cfg.group;

        # Hardening
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        PrivateDevices = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        RestrictNamespaces = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
      };
    };

    networking.firewall.allowedTCPPorts = lib.mkIf cfg.openFirewall [ cfg.port ];
  };
}
