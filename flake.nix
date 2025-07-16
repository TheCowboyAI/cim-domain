{
  description = "CIM Domain - Core DDD components and traits for CIM";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        rustVersion = pkgs.rust-bin.nightly."2024-11-01".default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        buildInputs = with pkgs; [
          openssl
          pkg-config
          protobuf
        ] ++ lib.optionals stdenv.isDarwin [
          darwin.apple_sdk.frameworks.Security
          darwin.apple_sdk.frameworks.SystemConfiguration
        ];

        nativeBuildInputs = with pkgs; [
          rustVersion
          cargo-edit
          cargo-watch
          cargo-audit
          cargo-outdated
          cargo-license
          rust-analyzer
        ];
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "cim-domain";
          version = "0.3.0";
          src = ./.;
          
          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit buildInputs nativeBuildInputs;

          checkType = "debug";
          doCheck = true;
        };

        devShells.default = pkgs.mkShell {
          inherit buildInputs;
          nativeBuildInputs = nativeBuildInputs ++ (with pkgs; [
            # Development tools
            tokei
            git
            jq
            
            # Testing tools
            cargo-nextest
            cargo-llvm-cov
            
            # Documentation
            mdbook
          ]);

          shellHook = ''
            echo "CIM Domain development environment"
            echo "Rust version: $(rustc --version)"
            echo ""
            echo "Available commands:"
            echo "  cargo build     - Build the project"
            echo "  cargo test      - Run tests"
            echo "  cargo doc       - Generate documentation"
            echo "  cargo bench     - Run benchmarks"
            echo ""
          '';
        };
      });
}