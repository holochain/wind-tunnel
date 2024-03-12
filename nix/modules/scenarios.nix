# Helper module to discover all scenarios and build packages for each.

{ self, inputs, lib, ... }@flake: {
  perSystem = { config, self', inputs', system, pkgs, ... }:
    let
      scenario_names = builtins.filter (name: !(lib.strings.hasInfix "." name)) (builtins.attrNames (builtins.readDir ../../scenarios));

      scenarios = map
        (scenario_name: {
          name = "scenario_${scenario_name}";
          value = config.scenarioHelper.mkScenario {
            name = scenario_name;
          };
        })
        scenario_names;
    in
    {
      packages = builtins.listToAttrs scenarios;
    };
}
