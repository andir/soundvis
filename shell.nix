with import <nixpkgs> {};
let
  mypy = python3.withPackages (ps: [ ps.matplotlib ps.numpy ]);
  xdeps = with xorg; [
    mesa
    libX11
    libXcursor
    libXrandr
    libinput
    libxcb
    libXi
  ];
  libdir = lib.makeLibraryPath xdeps;
in stdenv.mkDerivation {
  name = "soundvis";
  buildInputs = [ openssl ] ++ (with gst_all_1; [ gst-plugins-base gst-plugins-good protobuf ] ++ xdeps);
  LD_LIBRARY_PATH = libdir;
}
