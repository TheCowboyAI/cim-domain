# Copyright 2025 Cowboy AI, LLC.

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
        
        rustVersion = pkgs.rust-bin.nightly.latest.default.override {
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
        checks = {
          fmt = pkgs.runCommand "fmt-check" {
            nativeBuildInputs = nativeBuildInputs ++ (with pkgs; [ rustfmt ]);
          } ''
            export HOME=$TMPDIR
            cargo fmt --all -- --check
            touch $out
          '';

          clippy = pkgs.runCommand "clippy-check" {
            nativeBuildInputs = nativeBuildInputs ++ (with pkgs; [ clippy ]);
          } ''
            export HOME=$TMPDIR
            cargo clippy --workspace --all-features -- -D warnings
            touch $out
          '';

          tests = pkgs.runCommand "unit-tests" {
            nativeBuildInputs = nativeBuildInputs;
          } ''
            export HOME=$TMPDIR
            cargo test --workspace --all-features --locked -- --nocapture
            touch $out
          '';

          coverage = pkgs.runCommand "coverage-llvm-cov" {
            nativeBuildInputs = nativeBuildInputs ++ (with pkgs; [ cargo-llvm-cov llvmPackages.llvm ]);
          } ''
            export HOME=$TMPDIR
            # llvm-cov does runtime instrumentation; ptrace not required (works in sandbox)
            cargo llvm-cov --workspace --all-features --fail-under-lines 100 --no-report
            touch $out
          '';
        };

        apps = {
          # On-demand strict ACT/DDD verification gate (does not run in default checks)
          act-strict = {
            type = "app";
            program = pkgs.writeShellScriptBin "act-strict" ''
              set -euo pipefail
              export HOME=${TMPDIR:-/tmp}
              echo "Running strict ACT/DDD tests (feature: act_strict)"
              cargo test --features act_strict -- --nocapture
            '';
          };

          # TDD run: execute all tests but do not fail the app exit code (useful during red phase)
          tdd = {
            type = "app";
            program = pkgs.writeShellScriptBin "tdd" ''
              export HOME=${TMPDIR:-/tmp}
              echo "TDD mode: running tests and continuing even if failing..."
              set +e
              cargo test --workspace --all-features -- --nocapture
              code=$?
              echo "\n[TDD] cargo test exit code: $code (non-blocking)"
              exit 0
            '';
          };

          # UL tools (forward args)
          ul-dot = {
            type = "app";
            program = pkgs.writeShellScriptBin "ul-dot" ''
              set -euo pipefail
              cargo run -q -p domain_graph_tools --bin ul_dot -- "$@"
            '';
          };

          ul-narrative = {
            type = "app";
            program = pkgs.writeShellScriptBin "ul-narrative" ''
              set -euo pipefail
              cargo run -q -p domain_graph_tools --bin ul_narrative -- "$@"
            '';
          };

          add-morphism = {
            type = "app";
            program = pkgs.writeShellScriptBin "add-morphism" ''
              set -euo pipefail
              cargo run -q -p domain_graph_tools --bin add_morphism -- "$@"
            '';
          };

          add-diagram = {
            type = "app";
            program = pkgs.writeShellScriptBin "add-diagram" ''
              set -euo pipefail
              cargo run -q -p domain_graph_tools --bin add_diagram -- "$@"
            '';
          };

          # Render all DOT diagrams to SVG
          diagrams-render = {
            type = "app";
            program = pkgs.writeShellScriptBin "diagrams-render" ''
              set -euo pipefail
              shopt -s nullglob
              for f in doc/act/diagrams/*.dot; do
                echo "Rendering $f"
                dot -Tsvg "$f" -O
              done
            '';
          };

          # Attach planned diagrams with UL-aligned describes lists
          diagrams-attach = {
            type = "app";
            program = pkgs.writeShellScriptBin "diagrams-attach" ''
              set -euo pipefail
              cargo run -q -p domain_graph_tools --bin add_diagram -- --id event_pipeline_v2 --path doc/act/diagrams/event_pipeline_v2.dot.svg --describes handled_by,causes_event,emits_event,wraps_event,references_payload_cid,appended_to_stream,collects_envelope
              cargo run -q -p domain_graph_tools --bin add_diagram -- --id identity_envelope_v2 --path doc/act/diagrams/identity_envelope_v2.dot.svg --describes identified_by_command_id,encloses_command,command_carries_identity,identified_by_query_id,encloses_query,query_carries_identity,provides_correlation_id,provides_causation_id,provides_event_id,identifies_event,identifies_aggregate,correlates_with,was_caused_by,describes_payload,command_correlates_to_event,query_correlates_to_event,precedes_envelope,acknowledged_by_command,acknowledged_by_query
              cargo run -q -p domain_graph_tools --bin add_diagram -- --id read_path_v2 --path doc/act/diagrams/read_path_v2.dot.svg --describes subscribes_to_stream,consumes_event,updates_read_model,reads_from,responds_with
              cargo run -q -p domain_graph_tools --bin add_diagram -- --id addressing_v2 --path doc/act/diagrams/addressing_v2.dot.svg --describes domain_cid_defines_node,uses_payload_codec,payload_is,annotated_by_metadata,defined_by_ipld
              cargo run -q -p domain_graph_tools --bin add_diagram -- --id bounded_context_scope_v2 --path doc/act/diagrams/bounded_context_scope_v2.dot.svg --describes scopes_aggregate,scopes_projection,scopes_read_model,scopes_event_stream,scopes_command,scopes_query,scopes_policy,scopes_state_machine,scopes_saga
            '';
          };
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "cim-domain";
          version = "0.7.5";
          src = ./.;
          
          cargoLock = { lockFile = ./Cargo.lock; };

          inherit buildInputs nativeBuildInputs;

          checkType = "debug";
          doCheck = false;
          
          # Install the library
          postInstall = ''
            mkdir -p $out/lib
            cp target/*/release/libcim_domain.rlib $out/lib/ || true
            cp target/*/release/deps/libcim_domain*.rlib $out/lib/ || true
          '';
        };

        devShells = {
          default = pkgs.mkShell {
          inherit buildInputs;
          nativeBuildInputs = nativeBuildInputs ++ (with pkgs; [
            # Development tools
            tokei
            git
            jq
            graphviz
            
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
            
            # LLVM tools for coverage
            llvmPackages.bintools
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
            echo "  dot -Tsvg doc/act/diagrams/*.dot -O  - Render DOT to SVG"
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
            
            # Set up LLVM tools for cargo-llvm-cov
            export RUSTFLAGS="-C instrument-coverage"
            export LLVM_PROFILE_FILE="cim-domain-%p-%m.profraw"
            
            # Fix Node.js MaxListenersExceededWarning
            export NODE_OPTIONS="--max-listeners=50"
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
              graphviz
              
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
              
              # Fix Node.js MaxListenersExceededWarning
              export NODE_OPTIONS="--max-listeners=50"
            '';
          };
        };
      });
}
