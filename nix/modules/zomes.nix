# Helper module to discover all the zomes in the zomes directory and create a package for each one

{ config, lib, ... }:
let
  zome_names = builtins.filter (name: !(lib.strings.hasInfix "." name)) (builtins.attrNames (builtins.readDir ../../zomes));

  zome_names_with_types = builtins.map (zome_name: builtins.map (kind: { name = zome_name; inherit kind; }) (builtins.attrNames (builtins.readDir ../../zomes/${zome_name}))) zome_names;

  named_zomes = builtins.map ({ name, kind } @ input: { name = "${name}_${kind}"; value = config.zomeHelper.mkZome input; }) (lib.lists.flatten zome_names_with_types);
in
{
  packages = builtins.listToAttrs named_zomes;
}
