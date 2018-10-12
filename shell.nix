with import <nixpkgs> {};
let
  mypy = python3.withPackages (ps: [ ps.matplotlib ps.numpy ]);
  xdeps = with xorg; [
    libX11
    libXcursor
    libXrandr
    libinput
    libxcb
    libXi
  ];
  libdir = lib.makeLibraryPath (xdeps ++ [ "/run/opengl-driver"  libGLU_combined ]);
in stdenv.mkDerivation {
  name = "soundvis";
  buildInputs = [
    openssl gdb
  ] ++ (with gst_all_1; [
    gst-plugins-base gst-plugins-good protobuf
  ] ++ xdeps);
  LD_LIBRARY_PATH = libdir;
}
