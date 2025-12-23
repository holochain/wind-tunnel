{
  description = "A Nix flake for a jekyll development environment";

  inputs.nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.11";

  # utility to iterate over multiple target platforms
  inputs.flake-parts.url = "github:hercules-ci/flake-parts";

  outputs = inputs @ { self, nixpkgs, flake-parts, ... }:
    # refer to flake-parts docs https://flake.parts/
    flake-parts.lib.mkFlake { inherit inputs; }
      {
        # systems that his flake can be used on
        systems = [ "aarch64-darwin" "x86_64-linux" "x86_64-darwin" "aarch64-linux" ];

        # for each system...
        perSystem = { config, pkgs, system, ... }:
          let
            pkgs = import nixpkgs {
              inherit system;
            };
          in
          {
            # Configure a formatter so that `nix fmt` can be used to format this file.
            formatter = pkgs.nixpkgs-fmt;

            devShells = {
              default = pkgs.mkShell {
                packages = with pkgs; [
                  ruby_3_4
                  jekyll
                ];
              };
            };
          };
      };
}
