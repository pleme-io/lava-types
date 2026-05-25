# nix/modules/home-manager.nix — auto-generated from lava-types.caixa.lisp
{ config, lib, pkgs, ... }:
let cfg = config.programs.lava-types; in {
  options.programs.lava-types = {
    enable = lib.mkEnableOption "lava-types";
    package = lib.mkOption { type = lib.types.package; default = pkgs.lava-types or null; };
  };
  config = lib.mkIf cfg.enable { home.packages = [ cfg.package ]; };
}
