let
  env = import ./nix/environment.nix;
in env.pkgs.mkShell {
  buildInputs = env.dependencies ++ env.shellDependencies;
  shellHook = ''
    export SSL_CERT_FILE=${env.pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
  '';
}
