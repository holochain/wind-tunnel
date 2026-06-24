# Helper module to discover all scenarios and build packages for each.

{ config, lib, ... }:
let
  # Temporarily exclude unyt scenarios, until they're upgraded to Holochain v0.7.
  scenario_names = builtins.filter (name: !(lib.strings.hasInfix "." name) && !(lib.strings.hasPrefix "unyt_" name)) (builtins.attrNames (builtins.readDir ../../scenarios));

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
