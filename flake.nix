{
  description = "Gladius development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay.url = "github:oxalica/rust-overlay";
    systems = {
      url = "github:nix-systems/default";
      flake = false;
    };
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [];

      systems = import inputs.systems;

      perSystem = {
        self',
        pkgs,
        system,
        ...
      }: let
        rustToolchain = pkgs.rust-bin.stable.latest.default;
        tools = with pkgs; [
          just
          bacon
          cargo-nextest
          cargo-expand
          cargo-msrv
          cargo-binstall
          git-cliff
        ];
      in {
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [(import inputs.rust-overlay)];
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [rustToolchain];
          packages = [rustToolchain tools];
          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          shellHook = ''
            export PATH="$HOME/.cargo/bin:$PATH"
            echo "Installing committed..."
            cargo binstall --no-confirm committed
            echo "Installing git hooks..."
            if [ ! -f .git/hooks/commit-msg ]; then
              cat > .git/hooks/commit-msg << 'EOF'
            #!/usr/bin/env bash
            committed --commit-file "$1" --config .committed.toml
            EOF
              chmod +x .git/hooks/commit-msg
              echo "commit-msg hook installed"
            else
              echo "commit-msg hook already exists"
            fi
          '';
        };
      };
    };
}
