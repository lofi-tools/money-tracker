{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-25.05";
  inputs.parts.url = "github:hercules-ci/flake-parts";
  inputs.my-nix = { url = "github:nmrshll/nix-utils"; inputs.nixpkgs.follows = "nixpkgs"; inputs.fp.follows = "parts"; };


  outputs = inputs@{ self, parts, my-nix, ... }: parts.lib.mkFlake { inherit inputs; } (top@{ lib, ... }:
    with builtins; {
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];
      imports = (attrValues inputs.my-nix.flakeModules) ++ [ ];
      perSystem = { pkgs, system, lib, lib', self', ... }:
        let
          l = builtins // top.lib // lib // lib';
          bin = inputs.my-nix.bin.${system} // (mapAttrs (n: p: "${p}/bin/${n}") scripts);
          buildDeps = [
            pkgs.nodePackages_latest.pnpm
          ];
          devDeps = [
            pkgs.cargo-edit
            pkgs.watchexec
          ];

          wd = "$(git rev-parse --show-toplevel)";
          scripts = mapAttrs (n: s: pkgs.writeShellScriptBin n s) { };

          crates = {
            # new = l.customRust.buildCrate "new";
          };

          env = {
            RUST_LOG = "debug";
            RUST_BACKTRACE = 1;
          };
        in
        {
          packages = crates // { default = crates.new; } // scripts;
          devShellParts.env = env;
          devShellParts.buildInputs = buildDeps ++ devDeps ++ (attrValues scripts);
        };
    });
}

# {
#   inputs = {
#     nixpkgs.url = "github:nixos/nixpkgs/nixos-24.05";
#     utils.url = "github:numtide/flake-utils";
#     rust-overlay = {
#       url = "github:oxalica/rust-overlay";
#       inputs.nixpkgs.follows = "nixpkgs";
#       inputs.flake-utils.follows = "utils";
#     };
#     # crane = {
#     #   url = "github:ipetkov/crane";
#     #   inputs.nixpkgs.follows = "nixpkgs";
#     # };
#    my-utils={
#       url = "github:nmrshll/nix-utils";      
#       inputs.nixpkgs.follows = "nixpkgs";
#       inputs.utils.follows = "utils";
#       # inputs.rust-overlay.follows = "rust-overlay";
#     };
#   };

#   outputs = { self, nixpkgs, rust-overlay, utils, my-utils }:
#     with builtins; utils.lib.eachDefaultSystem (system:
#         let
#           pkgs = import nixpkgs {
#             inherit system;
#             overlays = [ (import rust-overlay) ];
#           };
#           customRust = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
#             extensions = [ "rust-src" "rust-analyzer" ];
#             targets = [ ];
#           });

#           buildDependencies = with pkgs; [
#             customRust
#             nodePackages_latest.pnpm
#           ] ++ (
#             lib.optionals stdenv.isDarwin [
#               darwin.apple_sdk.frameworks.Security
#               darwin.apple_sdk.frameworks.SystemConfiguration
#               darwin.apple_sdk.frameworks.CoreServices
#               darwin.apple_sdk.frameworks.CoreFoundation
#               darwin.apple_sdk.frameworks.Foundation
#               libiconv
#             ]
#           );
#           devDependencies = [
#             pkgs.cargo-edit
#             pkgs.watchexec
#           ];

#           env = rec {
#             RUST_LOG = "debug";
#             RUST_BACKTRACE = 1;
#           };

#           binaries = my-utils.binaries.${system};
#           scripts = attrValues my-utils.packages.${system} ++ [
#             (pkgs.writeScriptBin "run" ''cargo run'')
#             (pkgs.writeScriptBin "itest" ''cargo test -p integration -- $SINGLE_TEST --nocapture'')
#             (pkgs.writeScriptBin "utest" ''cargo test --workspace --lib -- $SINGLE_TEST --nocapture'')
#             # (pkgs.writeScriptBin "front" ''cd frontend; pnpm install; pnpm dev'')
#           ];

#         in
#         {
#           devShells.default = pkgs.mkShell {
#             buildInputs = buildDependencies ++ devDependencies ++ scripts;
#             shellHook = ''
#               ${binaries.configure-vscode}; 
#               dotenv
#             '';
#             inherit env;
#           };
#         }
#       );
# }
