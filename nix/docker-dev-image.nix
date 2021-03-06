let
  env = import ./environment.nix;
  pkgs = env.pkgs;

  passthroughScript = env.pkgs.writeScriptBin "passthrough-shell" ''
    #!${pkgs.bash}/bin/bash
    export PATH=${pkgs.lib.makeBinPath (with pkgs; [ shadow coreutils su ])}

    # Set up HOME passthrough
    # We use USER, GROUP, UID, GID, HOME
    echo "root:x:0:" > /etc/group
    echo "root:x:0:0:Rooooooot:/root:/bin/sh" > /etc/passwd

    echo "$GROUP:x:$GID:" >> /etc/group
    echo "$USER:x:$UID:$GID::$HOME:/bin/sh" >> /etc/passwd

    exec ${insecureBecome}/bin/become "$UID" "$GID" "$@"
  '';
  insecureBecome = pkgs.stdenv.mkDerivation {
    name = "insecure-become";
    src = ./insecure-become.c;
    unpackPhase = "true";
    buildPhase = ''
      gcc -Wall -Werror -o become $src
    '';
    installPhase = ''
      mkdir -p $out/bin
      cp become $out/bin
    '';
  };
in pkgs.dockerTools.buildImage {
  name = "redleaf-dev";
  contents = env.dependencies ++ env.shellDependencies ++ [ passthroughScript ];
  config = {
    Cmd = [ "bash" ];
    Env = [
      "TERMINFO_DIRS=${pkgs.ncurses}/share/terminfo"
    ];
  };
}
