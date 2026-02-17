{
  description = "Google Classroom MCP Server — Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            cargo
            rustc
            rustfmt
            clippy

            # Build dependencies
            pkg-config
            openssl
          ];

          shellHook = ''
            echo "🎓 google-classroom-mcp dev shell"
            echo "  Rust   : $(rustc --version)"
            echo "  Cargo  : $(cargo --version)"
            echo ""
            echo "Available commands:"
            echo "  gcm-dev   — cargo run -- run"
            echo "  gcm-build — cargo build --release"
            echo "  gcm-auth  — cargo run -- auth"
            echo "  gcm-test  — cargo test"

            gcm-dev()   { cargo run -- run "$@"; }
            gcm-build() { cargo build --release "$@"; }
            gcm-auth()  { cargo run -- auth "$@"; }
            gcm-test()  { cargo test "$@"; }

            export -f gcm-dev gcm-build gcm-auth gcm-test
          '';

          env = {
            RUST_LOG = "info";
          };
        };
      }
    );
}
