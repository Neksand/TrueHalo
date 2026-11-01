# Publishing `truehalo-bin` to the AUR

The package files (`PKGBUILD`, `.SRCINFO`) live here. Uploading needs an
[archlinux.org](https://archlinux.org) account with an SSH key registered on
the [AUR](https://aur.archlinux.org) — the maintainer's key, not CI.

```bash
# one time: accept the AUR host key, then clone the (empty) package repo
git clone ssh://aur@aur.archlinux.org/truehalo-bin.git
cp packaging/aur/PKGBUILD packaging/aur/.SRCINFO truehalo-bin/
cd truehalo-bin

# verify the checksum resolves against the release asset
makepkg --printsrcinfo > .SRCINFO   # must match the committed file
makepkg -si                          # builds nothing; repackages the .deb, installs locally

git add PKGBUILD .SRCINFO
git commit -m "truehalo-bin 0.3.1-1"
git push origin master
```

On every release: bump `pkgver`, refresh `sha256sums` with
`sha256sum TrueHalo_<ver>_amd64.deb`, regenerate `.SRCINFO`, test with
`makepkg -si`, push.
