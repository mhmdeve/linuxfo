# LinuxFo APT repository

This repository is published at `https://mhmdeve.github.io/linuxfo/` by the
`.github/workflows/apt-repository.yml` workflow. It hosts signed Debian packages
for `amd64` and `arm64`.

## One-time maintainer setup

1. Generate a dedicated OpenPGP signing key for the APT repository. Do not reuse
   a personal email-signing key. Keep a secure offline backup of the private key.
2. In GitHub, open **Settings → Secrets and variables → Actions** and add:
   - `APT_GPG_PRIVATE_KEY`: the complete ASCII-armored private key export.
   - `APT_GPG_PASSPHRASE`: its passphrase (omit this secret only if the key has
     no passphrase).
3. Open **Settings → Pages** and select **GitHub Actions** as the build/deployment
   source.
4. Publish a GitHub Release. The workflow will download its `linuxfo_*_amd64.deb`
   and `linuxfo_*_arm64.deb` assets and publish signed APT metadata. You can
   also run the workflow manually and enter an existing release tag.

Generate/export a dedicated key locally (protect the private export carefully):

```bash
gpg --full-generate-key
gpg --armor --export-secret-keys YOUR_KEY_ID
```

Paste the private export into the GitHub Actions secret, never into the repository.
The public key is published by the workflow at
`https://mhmdeve.github.io/linuxfo/linuxfo-archive-keyring.asc`.

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

The repository publishes packages built by the project's Debian package workflow.
Only the architectures listed in the repository Release metadata are supported.
