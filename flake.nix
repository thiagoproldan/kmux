{
  description = "kmux -- ztmux, a Rust port of tmux, under its own name";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      # Read rather than repeated, as ekko's flake does: the version is
      # upstream's, and moves with each merge from it.
      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
    in
    {
      packages = forAll (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "kmux";
          inherit (cargoToml.package) version;

          # `target/` is build output and would otherwise be copied into the
          # store on every evaluation.
          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = path: _type: baseNameOf path != "target";
          };

          cargoLock.lockFile = ./Cargo.lock;

          meta = with pkgs.lib; {
            inherit (cargoToml.package) description;
            homepage = cargoToml.package.repository;
            # ztmux's MIT notice (LICENSE) and tmux's ISC one (COPYING).
            license = [
              licenses.mit
              licenses.isc
            ];
            mainProgram = "kmux";
            platforms = platforms.linux;
          };
        };
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            clippy
            rustfmt
          ];
        };
      });
    };
}
