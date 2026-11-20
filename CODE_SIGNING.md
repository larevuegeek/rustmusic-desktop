# Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by
[SignPath Foundation](https://signpath.org).

## What is signed

The Windows releases of RustMusic: the program `rustmusic.exe` and its installer
`rustmusic_<version>_x64-setup.exe`. They are built by
[GitHub Actions](.github/workflows/release.yml) from the source code of this repository,
for a release tag, and each signing request is approved manually before the files are
signed. No binary built elsewhere is signed.

The macOS releases are signed with the maintainer's Apple Developer ID and notarized by
Apple. Every package of every system is also signed with the project's update key
(minisign), which RustMusic checks before installing an update.

## Team roles

- Committers and reviewers: [larevuegeek](https://github.com/larevuegeek)
- Approvers: [larevuegeek](https://github.com/larevuegeek)

Changes proposed by anyone else (pull requests) are reviewed by a committer before
they are merged.

## Privacy

See the [privacy policy](PRIVACY.md): what RustMusic sends over the network, to whom,
and how to turn it off.
