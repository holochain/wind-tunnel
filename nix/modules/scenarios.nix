# Helper module to discover all scenarios and build packages for each.

{ config, lib, ... }:
let
  scenario_names = builtins.filter (name: !(lib.strings.hasInfix "." name)) (builtins.attrNames (builtins.readDir ../../scenarios));

  scenarios = map
    (name: {
      inherit name;
      value = config.scenarioHelper.mkScenario {
        inherit name;
      };
    })
    scenario_names;
in
{
  packages = builtins.listToAttrs scenarios;
}
