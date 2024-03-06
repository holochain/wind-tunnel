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
                    pkgs.yq
                    pkgs.curl
                ];

                shellHook = ''
                    # Configure InfluxDB to store data within the repository
                    export INFLUXD_BOLT_PATH="`pwd`/influx/.influxdbv2/influxd.bolt"
                    export INFLUXD_ENGINE_PATH="`pwd`/influx/.influxdbv2/engine/"
                    export INFLUXD_CONFIG_PATH="`pwd`/influx/"

                    # Configure the InfluxDB CLI to store its config within the repository
                    export INFLUX_CONFIGS_PATH="`pwd`/influx/influx.toml"

                    # Configures the current shell to use InfluxDB with Wind Tunnel
                    use_influx() {
                        export INFLUX_HOST="http://localhost:8087"
                        export INFLUX_BUCKET=windtunnel
                        export INFLUX_TOKEN="$(cat $INFLUX_CONFIGS_PATH | tomlq -r .default.token)"
                    }

                    # Dev only setup for InfluxDB, this function can be called from inside the dev shell once `influxd` is running
                    configure_influx() {
                        influx setup --host http://localhost:8087 --username windtunnel --password windtunnel --org holo --bucket windtunnel --force
                        use_influx

                        # Import variables
                        ls influx/templates/variables/ | xargs -I % influx apply --host "$INFLUX_HOST" --token "$INFLUX_TOKEN" --org holo --file "`pwd`/influx/templates/variables/%" -quiet --force yes

                        # Import dashboards
                        ls influx/templates/dashboards/ | xargs -I % influx apply --host "$INFLUX_HOST" --token "$INFLUX_TOKEN" --org holo --file "`pwd`/influx/templates/dashboards/%" --quiet --force yes
                    }

                    # Remove data and config
                    clear_influx() {
                         curl "http://localhost:8087/debug/flush"
                         rm "$INFLUX_CONFIGS_PATH"
                    }
                '';
            };
        };
    };
}
