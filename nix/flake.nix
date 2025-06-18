{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" ];
          };
          nativeBuildInputs = [
            rustToolchain
            pkgs.pkg-config
            pkgs.just
            pkgs.nodejs_24
          ];
          buildInputs = [
            pkgs.dioxus-cli
            pkgs.gtk3
            pkgs.webkitgtk_4_1
            pkgs.xdotool
          ];
        in
        with pkgs;
        {
          devShells.default = mkShell {
            inherit nativeBuildInputs buildInputs;

            # fix for font size
            shellHook = ''
              export XDG_DATA_DIRS=${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:$XDG_DATA_DIRS;
              export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules/";
            '';
          };
        }
      );
}
