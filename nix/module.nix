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

    dataDir = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/cookit";
      description = "Directory where the sqlite database and other state are stored.";
    };

    cookitUrl = lib.mkOption {
      type = lib.types.str;
      description = "Public URL";
      example = "https://cookit.example.com";
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
      home = cfg.dataDir;
    };

    users.groups.${cfg.group} = { };

    systemd.services.cookit = {
      description = "CookIt recipes";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      environment = {
        DATABASE_URL = "sqlite://${cfg.dataDir}/cookit.db?mode=rwc";
        IP = cfg.ipAddress;
        PORT = toString cfg.port;
        URL = cfg.cookitUrl;
        LOG_LEVEL = cfg.logLevel;
      };

      serviceConfig = {
        ExecStart = "${cfg.package}/bin/web";
        User = cfg.user;
        Group = cfg.group;
        WorkingDirectory = cfg.dataDir;

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

        # Ensure dataDir exists and is writable by the service user.
        ReadWritePaths = [ cfg.dataDir ];
        StateDirectory = lib.mkIf (cfg.dataDir == "/var/lib/cookit") "cookit";
      };
    };

    networking.firewall.allowedTCPPorts = lib.mkIf cfg.openFirewall [ cfg.port ];

    systemd.tmpfiles.rules = lib.mkIf (cfg.dataDir != "/var/lib/cookit") [
      "d ${cfg.dataDir} 0750 ${cfg.user} ${cfg.group} - -"
    ];
  };
}
