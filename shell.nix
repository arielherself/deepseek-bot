with import <nixpkgs>{};
pkgs.mkShell {
  packages = with pkgs; [
    rustc
    cargo
    clippy
    rust-analyzer
    libiconv
    openssl
    pkg-config
    python311
  ];
}
