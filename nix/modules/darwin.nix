# nix/modules/darwin.nix — auto-generated from lava-types.caixa.lisp
{ config, lib, pkgs, ... }:
let cfg = config.services.lava-types; in {
  options.services.lava-types = {
    enable = lib.mkEnableOption "lava-types";
    package = lib.mkOption { type = lib.types.package; default = pkgs.lava-types or null; };
  };
  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
  };
}
