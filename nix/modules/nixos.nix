# nix/modules/nixos.nix — auto-generated from lava-types.caixa.lisp
# description: "Typed constraint validators for lava architectures. Pangea Dry::Struct analog: CIDR / port / protocol / enum / regex / length / range / IPv4 / IPv6 / hostname. Validated at composition time (not apply time) — invalid CIDR fails the plan, not the cloud API. Pangea Types::String.constrained(...) → lava (:type :cidr-block) etc."
{ config, lib, pkgs, ... }:
let
  cfg = config.services.lava-types;
in {
  options.services.lava-types = {
    enable = lib.mkEnableOption "lava-types";
    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.lava-types or null;
    };
  };
  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
  };
}
