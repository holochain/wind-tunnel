{ pkgs
, lib
, scenarioName
}:
default:
fn:
let
  # Get the TOML object from the Cargo.toml file for the passed scenario
  cargoToml = lib.importTOML ../../scenarios/${scenarioName}/Cargo.toml;
  # If there are hApps to fetch then return a list of them, otherwise return an empty list
  hAppsToFetch = lib.lists.toList (lib.attrsets.attrByPath [ "package" "metadata" "fetch-required-happ" ] [ ] cargoToml);
  # Convert the passed hApp into a source to be fetched from a URL
  hAppSource = hApp: builtins.fetchurl { inherit (hApp) name url sha256; };
  # Create a derivation that stores all of the fetched hApps
  allFetchedHApps = pkgs.stdenv.mkDerivation {
    name = "${scenarioName}-fetched-hApps";
    dontUnpack = true;
    installPhase = ''
      mkdir -p $out
      ${lib.strings.concatMapStringsSep "\n" (hApp: "cp ${hAppSource hApp} $out/${hApp.name}.happ") hAppsToFetch}
    '';
  };
in
# If there are hApps to fetch then call the function with the path, else return the default value
if hAppsToFetch != [ ] then fn allFetchedHApps else default
