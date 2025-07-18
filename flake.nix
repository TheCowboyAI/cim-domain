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

        devShells = {
          default = pkgs.mkShell {
          inherit buildInputs;
          nativeBuildInputs = nativeBuildInputs ++ (with pkgs; [
            # Development tools
            tokei
            git
            jq
            
            # Testing tools
            cargo-nextest
            cargo-llvm-cov
            cargo-tarpaulin
            
            # Documentation
            mdbook
            
            # Code quality tools
            clippy
            rustfmt
            
            # Debugging tools
            gdb
            lldb
          ]);

          shellHook = ''
            echo "CIM Domain development environment"
            echo "Rust version: $(rustc --version)"
            echo ""
            echo "Available commands:"
            echo "  cargo build       - Build the project"
            echo "  cargo test        - Run tests"
            echo "  cargo doc         - Generate documentation"
            echo "  cargo bench       - Run benchmarks"
            echo "  cargo clippy      - Run linter"
            echo "  cargo fmt         - Format code"
            echo "  cargo nextest run - Run tests with nextest"
            echo "  cargo tarpaulin   - Generate test coverage report"
            echo "  cargo llvm-cov    - Generate LLVM coverage report"
            echo ""
            echo "Test coverage commands:"
            echo "  cargo tarpaulin --lib                    - Library coverage"
            echo "  cargo tarpaulin --all-features           - Full coverage"
            echo "  cargo tarpaulin --out Html               - HTML report"
            echo "  cargo llvm-cov --html                    - LLVM HTML report"
            echo ""
          '';
          };
          
          # Specialized shell for test coverage and quality checks
          test = pkgs.mkShell {
            inherit buildInputs;
            nativeBuildInputs = nativeBuildInputs ++ (with pkgs; [
              # Coverage tools
              cargo-tarpaulin
              cargo-llvm-cov
              grcov
              
              # Testing tools
              cargo-nextest
              cargo-insta
              
              # Quality tools
              clippy
              cargo-audit
              cargo-outdated
              cargo-deny
              cargo-machete
              
              # Benchmarking
              cargo-criterion
              hyperfine
              
              # Utilities
              jq
              yq-go
              git
            ]);
            
            shellHook = ''
              echo "CIM Domain Test & Coverage Environment"
              echo ""
              echo "Coverage commands:"
              echo "  cargo tarpaulin --lib --out Html         - Generate HTML coverage report for library"
              echo "  cargo tarpaulin --all-features --out Xml  - Generate XML coverage report"
              echo "  cargo llvm-cov --html                     - LLVM coverage with HTML output"
              echo "  cargo llvm-cov --lcov --output-path lcov.info - Generate LCOV report"
              echo ""
              echo "Quality checks:"
              echo "  cargo clippy -- -D warnings               - Strict linting"
              echo "  cargo audit                               - Security audit"
              echo "  cargo outdated                            - Check for outdated dependencies"
              echo "  cargo deny check                          - Check dependency licenses and security"
              echo "  cargo machete                             - Find unused dependencies"
              echo ""
              echo "Testing:"
              echo "  cargo nextest run                         - Fast parallel test runner"
              echo "  cargo test --doc                          - Run doctests"
              echo "  cargo test --all-features                 - Test with all features"
              echo ""
              echo "Benchmarking:"
              echo "  cargo criterion                           - Run criterion benchmarks"
              echo "  cargo bench                               - Run standard benchmarks"
              echo ""
            '';
          };
        };
      });
}