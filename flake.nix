{
    description = "Flake for Holochain testing";

    inputs = {
        versions.url = "github:holochain/holochain?dir=versions/0_2";

        versions.inputs.holochain.url = "github:holochain/holochain/holochain-0.2.6";

        holochain = {
            url = "github:holochain/holochain";
            inputs.versions.follows = "versions";
        };

        nixpkgs.follows = "holochain/nixpkgs";
    };

    outputs = inputs @ { ... }:
    inputs.holochain.inputs.flake-parts.lib.mkFlake { inherit inputs; }
    {
        systems = builtins.attrNames inputs.holochain.devShells;
        perSystem = { config, pkgs, system, ... }: {
            devShells.default = pkgs.mkShell {
                inputsFrom = [
                    inputs.holochain.devShells.${system}.holonix
                ];
                packages = [
                    pkgs.influxdb2-cli
                    pkgs.influxdb2-server
                ];

                shellHook = ''
                    export INFLUXD_BOLT_PATH=`pwd`/influx/.influxdbv2/influxd.bolt
                    export INFLUXD_ENGINE_PATH=`pwd`/influx/.influxdbv2/engine
                    export INFLUXD_CONFIG_PATH=`pwd`/influx
                '';
            };
        };
    };
}
