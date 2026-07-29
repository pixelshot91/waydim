{
  description = "waydim - hybrid hardware + software display dimmer (Wayland compatible)",
  inputs = {},
  outputs = { self, ... }: {
    packages.x86_64-linux.default = pkgs: pkgs.rustPackages.rustc;
  }
}
