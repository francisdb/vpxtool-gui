{ pkgs ? import <nixpkgs> {} }:
  pkgs.mkShell {
    nativeBuildInputs = with pkgs.buildPackages; [ pkg-config ];

    buildInputs = with pkgs; [
      udev
      alsa-lib
    ];
    shellHook = ''
        export RUST_BACKTRACE=1
    '';
}
