# Module for building hApps and DNAs from the requirements specified in the metadata section of a Cargo.toml

{ self, inputs, lib, ... }@flake: {
  perSystem = { config, self', inputs', system, pkgs, ... }:
    let
      inherit (config.rustHelper) craneLib;

      zomeHasIntegrity = zome_name: builtins.hasAttr "${zome_name}_integrity" self'.packages;
    in
    {
      options.happHelper = lib.mkOption { type = lib.types.raw; };

      config.happHelper = {
        mkHapps = { configToml }:
          let
            config = builtins.fromTOML (builtins.readFile configToml);
            inherit (config.package) metadata;

            requiredDnas =
              if builtins.hasAttr "required-dna" metadata then
                if builtins.isList metadata."required-dna" then
                  metadata."required-dna"
                else
                  [ metadata."required-dna" ]
              else
                [ ];

            #
            #
            # This section contains DNA build logic
            #
            #

            mkDnaManifest = { name, zomes }: {
              manifest_version = "1";
              inherit name;

              coordinator = {
                zomes = builtins.map
                  (zome_name:
                    {
                      name = zome_name;
                      hash = null;
                      # Relying on referencing a derivation here and having Nix convert that to the store path of that derivation when it's converted to a string
                      bundled = "${self'.packages."${zome_name}_coordinator"}/lib/${zome_name}_coordinator.wasm";
                      dependencies =
                        if builtins.hasAttr "${zome_name}_integrity" self'.packages then
                          [{ name = "${zome_name}_integrity"; }]
                        else
                          [ ];
                      dylib = null;
                    }
                  )
                  zomes;
              };

              integrity = {
                network_seed = null;
                properties = null;
                origin_time = 1710431275;
                zomes = builtins.map
                  (zome_name: {
                    name = "${zome_name}_integrity";
                    hash = null;
                    # Relying on referencing a derivation here and having Nix convert that to the store path of that derivation when it's converted to a string
                    bundled = "${self'.packages."${zome_name}_integrity"}/lib/${zome_name}_integrity.wasm";
                    dependencies = null;
                    dylib = null;
                  })
                  (builtins.filter zomeHasIntegrity zomes);
              };
            };

            mkDnaBuildScript = manifest: ''
              mkdir -p ./${manifest.name}
              echo '${lib.generators.toYAML {} manifest}' > ./${manifest.name}/dna.yaml

              hc dna pack --output ./${manifest.name}.dna ./${manifest.name}
            '';

            mkDnaBuildScripts = manifests:
              builtins.concatStringsSep "\n" (map mkDnaBuildScript manifests);

            #
            #
            # This section contains hApp build logic
            #
            #

            requiredHapps =
              if builtins.hasAttr "required-happ" metadata then
                if builtins.isList metadata."required-happ" then
                  metadata."required-happ"
                else
                  [ metadata."required-happ" ]
              else
                [ ];

            mkHappManifest = { name, dnas }: {
              manifest_version = "1";
              inherit name;
              description = "A Wind Tunnel sample hApp";

              roles = builtins.map
                (dna_name:
                  {
                    name = dna_name;

                    provisioning = {
                      strategy = "create";
                      deferred = false;
                    };

                    dna = {
                      # These are built as part of this derivation so look for them in the build directory
                      bundled = "../${dna_name}.dna";
                    };

                    modifiers = {
                      network_seed = null;
                      properties = null;
                      origin_time = null;
                      quantum_time = null;
                    };
                    installed_hash = null;
                    clone_limit = 0;
                  }
                )
                dnas;
            };

            mkHappBuildScript = manifest: ''
              mkdir -p ./${manifest.name}
              echo '${lib.generators.toYAML {} manifest}' > ./${manifest.name}/happ.yaml

              mkdir -p $out/${manifest.name}
              hc app pack --output $out/${manifest.name}.happ ./${manifest.name}
            '';

            mkHappBuildScripts = manifests:
              builtins.concatStringsSep "\n" (map mkHappBuildScript manifests);

            #
            #
            # Figure out what zome packages are required as inputs to this derivation
            #
            #
            requiredZomes = lib.lists.unique (lib.lists.flatten (map (builtins.getAttr "zomes") requiredDnas));

            # The zome packages that this derivation depends on
            zomeDeps = builtins.map (zome_name: [ self'.packages."${zome_name}_coordinator" ] ++ (if zomeHasIntegrity zome_name then [ self'.packages."${zome_name}_integrity" ] else [ ])) requiredZomes;
          in
          pkgs.stdenv.mkDerivation {
            inherit (config.package) name;

            # This is all based on workspace code, so rely on the Crane filter to select the right sources.
            src = craneLib.cleanCargoSource (craneLib.path ./../..);

            buildInputs = zomeDeps ++ [
              # Need `hc` to package DNAs and hApps.
              inputs.holochain.packages.${system}.holochain
            ];

            postInstall = ''
              # Ensure the derivation is not empty if there are ne hApps requested
              mkdir -p $out
              touch $out/.happ-build

              ${mkDnaBuildScripts (map mkDnaManifest requiredDnas)}
              ${mkHappBuildScripts (map mkHappManifest requiredHapps)}
            '';
          };
      };
    };
}
