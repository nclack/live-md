{
  pkgs,
  craneLib,
}: let
  inherit (pkgs) lib;

  # Update toolchain to ensure llvm-tools-preview is available
  toolchain = pkgs.rust-bin.stable.latest.default.override {
    extensions = ["llvm-tools-preview" ];
  };

  # Create a custom crane lib instance that uses our toolchain
  craneLibWithTools = craneLib.overrideToolchain toolchain;

  # Common arguments that will be used for both the package and checks
  src = lib.cleanSource ../.;

  commonArgs = {
    inherit src;
    nativeBuildInputs = with pkgs; [
      # Add llvm tools to all builds
      llvmPackages.bintools
      pkg-config
      openssl
    ];
  };

  # Build dependencies separately to improve caching
  cargoArtifacts = craneLibWithTools.buildDepsOnly commonArgs;

  # Main package
  package = craneLibWithTools.buildPackage (commonArgs
    // {
      inherit cargoArtifacts;
    });

  coverage = craneLibWithTools.cargoTest (commonArgs
    // {
      inherit cargoArtifacts;
      RUSTFLAGS = "-Cinstrument-coverage";
      LLVM_PROFILE_FILE = "coverage-%p-%m.profraw";
      CARGO_INCREMENTAL = "0";

      nativeBuildInputs =
        commonArgs.nativeBuildInputs
        ++ [
          pkgs.cargo-llvm-cov
          toolchain 
        ];

      buildPhase = ''
        # First run the tests with coverage instrumentation
        cargo llvm-cov test --workspace >> coverage-summary.txt

        # Generate all report formats
        cargo llvm-cov report --html 

        cargo llvm-cov report \
          --lcov \
          --output-path lcov.info

        cargo llvm-cov report \
          --codecov \
          --output-path codecov.json
      '';

      installPhase = ''
        pwd
        mkdir -p $out
        cp -r target/llvm-cov/html $out/
        cp lcov.info $out/
        cp codecov.json $out/
        cp coverage-summary.txt $out/
      '';
    });
in {
  inherit package toolchain coverage;

  checks = {
    # Inherit the main package as a check
    inherit package;

    # Add clippy check
    clippy = craneLibWithTools.cargoClippy (commonArgs
      // {
        inherit cargoArtifacts;
        cargoClippyExtraArgs = "--all-targets -- --deny warnings";
      });

    # Add rustfmt check
    fmt = craneLibWithTools.cargoFmt {
      inherit src;
    };

    # # Add cargo test
    # test = craneLibWithTools.cargoTest (commonArgs
    #   // {
    #     inherit cargoArtifacts;
    #     nativeBuildInputs = commonArgs.nativeBuildInputs ++ [
    #       pkgs.cargo-llvm-cov
    #       toolchain
    #     ];
    #     postInstall = ''
    #       cargo llvm-cov \
    #         --html \
    #         --ignore-filename-regex "/*" \
    #         --fail-under-lines 10
    #     '';
    #   });
  };
}
