# Ulanzi Deck plugins

Plugins for [Ulanzi Deck](https://github.com/Tyaaa-aa/Ulanzi-Deck-Linux), including the official
ones. Ulanzi Deck installs plugins from this repository's releases and verifies each package's
signature before installing it.

| Plugin | Description |
|---|---|
| [OBS Studio](plugins/obs/README.md) | Scenes, recording, streaming, replay buffer, virtual camera and audio, with live state on keys |
| [Home Assistant](plugins/ha/README.md) | Lights, fans, air conditioners and any entity: toggle, levels, scenes, live state and ready-made keys |

## Repository layout

```text
plugins/<id>/        one plugin: plugin.toml, src/, assets/ (icons), tests/, README.md
tests/host/          integration tests in the real plugin host, against mock OBS and Home Assistant
tools/release/       package signing and index generation (used by CI)
scripts/             package-plugin.sh (build and package one plugin), release.sh (CI)
```

Plugins are WebAssembly components built with the `ulanzi-plugin` SDK from the app repository. The
workspace pins the app commit that plugins are built and tested against (`[workspace.dependencies]`
in `Cargo.toml`); update the pin when a plugin needs newer app features. The contract, manifest
format and SDK are documented in the app's
[plugin guide](https://github.com/Tyaaa-aa/Ulanzi-Deck-Linux/blob/main/docs/plugins.md).

## Development

```bash
cargo test --workspace                       # unit tests, then each plugin in the real host
scripts/package-plugin.sh plugins/ha         # dist/plugins/ha-<version>.ulanziplugin
ulanzictl plugin install dist/plugins/ha-<version>.ulanziplugin   # unsigned: needs Developer Mode
```

Set `contract` and `min_app` in `plugin.toml` to match what the plugin uses. App versions older than
`min_app` refuse to install the plugin and report the required version.

## Releasing

Releases are fully automated. To release a plugin, increase `version` in its `plugin.toml` and merge
to `main`:

1. CI runs the tests.
2. For each plugin version without a release, CI builds and signs the package and publishes the
   release `<id>-v<version>` with `<id>-<version>.ulanziplugin` and its `.minisig`.
3. CI rebuilds `index.json` (the current version of each plugin, with each package's SHA-256), signs
   it and uploads both to the `index` release, which the app reads.

Pull requests are tested but never signed.

## Signing

Packages and the index are signed with [minisign](https://jedisct1.github.io/minisign/) (Ed25519).
The app includes the public key:

```text
RWT2/HtBKUuAWImA/92lW+GmNaz2nfsNJrxbGrfVmSj/sPyQlngs0pCf
```

CI signs with the secret key and its password, stored as the Actions secrets `MINISIGN_KEY` and
`MINISIGN_PASSWORD`. Workflows can use them, but their values cannot be read back. To verify a
download manually:

```bash
minisign -Vm ha-1.2.0.ulanziplugin -P RWT2/HtBKUuAWImA/92lW+GmNaz2nfsNJrxbGrfVmSj/sPyQlngs0pCf
```

## Official and third-party plugins

Each plugin declares its `author` in `plugin.toml`, and the app shows it as "by <author>". Plugins
maintained by the Ulanzi Deck project are listed in [`OFFICIAL`](OFFICIAL). Only the signed store
index marks plugins as **Official**, based on that file, so a plugin cannot declare itself official.
Only maintainers can change `OFFICIAL`.

## Submitting a plugin

Open a pull request that adds `plugins/<id>/` with the source, `plugin.toml`, icons in `assets/`,
tests and a README. CI builds and tests it, and once merged it is signed and listed in the store.

## License

MIT. Icon credits are in each plugin's `assets/CREDITS.md`.
