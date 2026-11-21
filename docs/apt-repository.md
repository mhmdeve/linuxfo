# LinuxFo APT repository

LinuxFo publishes a signed Debian repository at:

`https://mhmdeve.github.io/linuxfo/`

The repository currently supports Debian/Ubuntu systems using `amd64` and
`arm64`. It is built from the `.deb` files attached to a published GitHub
Release; the release workflow is the sole producer of those packages.

## One-time maintainer setup

1. Create a dedicated OpenPGP signing key for this APT repository. Do not reuse
   a personal email-signing key. Keep an offline backup of the private key.
2. In GitHub, open **Settings → Secrets and variables → Actions** and add:
   - `APT_GPG_PRIVATE_KEY`: the complete ASCII-armored private-key export.
   - `APT_GPG_PASSPHRASE`: the key passphrase. This may be omitted only for an
     unprotected key.
3. Open **Settings → Pages** and select **GitHub Actions** as the deployment
   source.
4. Ensure the `github-pages` environment permits deployments from the
   repository's default branch.

Generate/export the key locally (protect the private export carefully):

```bash
gpg --full-generate-key
gpg --armor --export-secret-keys YOUR_KEY_ID
```

Paste the private export into the GitHub Actions secret. Never commit it to the
repository or include it in a release asset. The workflow publishes only the
public key at
`https://mhmdeve.github.io/linuxfo/linuxfo-archive-keyring.asc`.

## Publishing

The normal process is:

1. Update `[workspace.package].version` in the root `Cargo.toml`.
2. Update `CHANGELOG.md` for that version.
3. Push a matching annotated tag, for example `v0.2.1`.
4. The **LinuxFo Release** workflow verifies that the tag matches the Cargo
   version, builds both architectures, packages the GUI and daemon into
   `amd64` and `arm64` Debian packages, and publishes the GitHub Release.
5. The **Publish LinuxFo APT repository** workflow runs when that release is
   published and deploys the signed package index to GitHub Pages.

You can manually rerun **LinuxFo Release** from Actions for the current Cargo
version. APT publishing can also be manually dispatched with an existing
version tag. It fails before deployment if either architecture package or the
signing key is missing or invalid.

## Install on Debian/Ubuntu

```bash
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL https://mhmdeve.github.io/linuxfo/linuxfo-archive-keyring.asc \
  | sudo gpg --dearmor --yes -o /etc/apt/keyrings/linuxfo.gpg
sudo chmod 0644 /etc/apt/keyrings/linuxfo.gpg
echo "deb [arch=amd64,arm64 signed-by=/etc/apt/keyrings/linuxfo.gpg] https://mhmdeve.github.io/linuxfo stable main" \
  | sudo tee /etc/apt/sources.list.d/linuxfo.list
sudo apt update
sudo apt install linuxfo
```

Direct `.deb` downloads remain available from the
[GitHub Releases page](https://github.com/mhmdeve/linuxfo/releases).
