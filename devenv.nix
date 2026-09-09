{
  pkgs,
  lib,
  config,
  inputs,
  ...
}: {
  # https://devenv.sh/languages/
  languages.rust.enable = true;
  languages.nix.enable = true;
}
