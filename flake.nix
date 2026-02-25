{
  description = "Personal Google MCP Server — Rust";

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
            echo "🎓 personal-google-mcp dev shell"
            echo "  Rust   : $(rustc --version)"
            echo "  Cargo  : $(cargo --version)"
            echo ""
            echo "Available commands:"
            echo "  pgm-dev   — cargo run -- run"
            echo "  pgm-build — cargo build --release"
            echo "  pgm-auth  — cargo run -- auth"
            echo "  pgm-test  — cargo test"

            pgm-dev()   { cargo run -- run "$@"; }
            pgm-build() { cargo build --release "$@"; }
            pgm-auth()  { cargo run -- auth "$@"; }
            pgm-test()  { cargo test "$@"; }

            export -f pgm-dev pgm-build pgm-auth pgm-test
          '';

          env = {
            RUST_LOG = "info";
          };
        };
      }
    );
}
