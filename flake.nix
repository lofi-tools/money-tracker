{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-23.11";
    utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };
    # crane = {
    #   url = "github:ipetkov/crane";
    #   inputs.nixpkgs.follows = "nixpkgs";
    # };
   my-utils={url = "github:nmrshll/nix-utils";      
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.utils.follows = "utils";
      # inputs.rust-overlay.follows = "rust-overlay";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, utils, my-utils }:
    with builtins; utils.lib.eachDefaultSystem (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [              (import rust-overlay)            ];
          };
          customRust = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
            extensions = [ "rust-src" "rust-analyzer" ];
            targets = [ ];
          });

          buildDependencies = with pkgs; [
            customRust
            nodePackages_latest.pnpm
          ] ++ (
            lib.optionals stdenv.isDarwin [
              darwin.apple_sdk.frameworks.Security
              darwin.apple_sdk.frameworks.SystemConfiguration
              darwin.apple_sdk.frameworks.CoreServices
              darwin.apple_sdk.frameworks.CoreFoundation
              darwin.apple_sdk.frameworks.Foundation
              libiconv
            ]
          );
          devDependencies = [
            pkgs.cargo-edit
            pkgs.watchexec
            # pkgs.sqlx-cli -> moved to sqlx-cli 0.7 which is not available on nix
          ];

          env = rec {
            RUST_LOG = "debug";
            RUST_BACKTRACE = 1;
            DATABASE_URL = "postgres://api:api@localhost:5454/api";
          };

          binaries = my-utils.binaries.${system};
          scripts = attrValues my-utils.packages.${system} ++ [
            (pkgs.writeScriptBin "back" ''cargo run'')
            (pkgs.writeScriptBin "tests" ''cargo test --test integration -- --nocapture'')
            (pkgs.writeScriptBin "utests" ''cargo test --lib -- --nocapture'')
            (pkgs.writeScriptBin "front" ''cd frontend; pnpm install; pnpm dev'')
          ];

        in
        {
          devShells.default = pkgs.mkShell {
            buildInputs = buildDependencies ++ devDependencies ++ scripts;
            shellHook = ''
              ${binaries.configure-vscode}; 
              dotenv
            '';
            inherit env;
          };
        }
      );
}
