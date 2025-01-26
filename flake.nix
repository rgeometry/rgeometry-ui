{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    worker-build.url = "github:lemmih/nix-flakes?dir=worker-build";
    wrangler.url = "github:ryand56/wrangler";
  };

  outputs = { self, nixpkgs, utils, naersk, worker-build, wrangler }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        naersk-lib = pkgs.callPackage naersk { };
        worker-build-bin = worker-build.packages.${system}.default;
        wrangler-bin = wrangler.packages.${system}.default;
      in {
        # defaultPackage = naersk-lib.buildPackage ./.;
        devShell = with pkgs; mkShell {
          buildInputs = [ cargo rustc rustfmt pre-commit rustPackages.clippy lld wasm-pack worker-build-bin wrangler-bin ];
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
        };
      }
    );
}
