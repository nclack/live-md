{
  pkgs,
  toolchain,
}: let
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
  fishConfig = pkgs.writeText "devshell-config.fish" ''
    function fish_right_prompt
        echo -n (set_color blue)"[${cargoToml.package.name}] "(set_color normal)
    end
    alias ls eza
    alias find fd

    # coverage helper functions
    function generate_coverage
        cargo llvm-cov clean    # Clean previous coverage data
        cargo llvm-cov --html   # Run tests and generate HTML report
        echo "Coverage report generated in ./target/llvm-cov/html/index.html"
    end

    function watch_coverage
        set -l port 8080
        if set -q argv[1]
            set port $argv[1]
        end
        echo "Starting coverage watch and live-server on port $port..."

        function cleanup
          echo "Cleaning up processes..."
          pkill -f "caddy file-server --listen :$port"
          pkill cargo # TODO: make this so it doesn't nuke all cargo's
        end

        trap cleanup EXIT
        trap cleanup INT

        mkdir -p target/llvm/html
        caddy file-server --listen :$port --root target/llvm-cov/html &

        cargo watch -x "llvm-cov --html" &
    end
  '';
in
  pkgs.mkShell {
    buildInputs = with pkgs; [
      # Rust toolchain from overlay
      bacon
      cargo-watch
      cargo-llvm-cov
      rust-analyzer
      toolchain

      # Development tools
      git
      helix
      markdown-oxide
      marksman
      nil
      xclip
      xsel
      caddy # live server for watching coverage
      cachix

      # Shell utilities
      eza
      fd
      fish
      fzf
      ripgrep
      tree

      # Debug and profiling tools
      clang-tools
      heaptrack
      hyperfine
      lldb
      perf-tools
      valgrind
    ];

    shellHook = ''
      export RUST_BACKTRACE=1
      export RUSTFLAGS="-Cinstrument-coverage -C debuginfo=2"
      export LLVM_PROFILE_FILE="coverage-%p-%m.profraw"
      export CARGO_INCREMENTAL=0
      export EDITOR=hx
      exec fish -C "source ${fishConfig}"
    '';

    interactiveShellInit = ''
      fzf_key_bindings
      set -U FZF_DEFAULT_OPTS "--height 40% --layout=reverse --border"
    '';
  }
