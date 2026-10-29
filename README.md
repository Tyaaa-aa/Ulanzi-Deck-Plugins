# Ulanzi Deck plugins

Every plugin for [Ulanzi Deck](https://github.com/Tyaaa-aa/Ulanzi-Deck-Linux) lives here, the
first-party ones included. The app installs them from this repo's releases and checks each package's
signature first.

| Plugin | What it does |
|---|---|
| [OBS Studio](plugins/obs/README.md) | Scenes, recording, streaming, replay buffer, virtual camera and audio, with live state on keys |
| [Home Assistant](plugins/ha/README.md) | Lights, fans, air conditioners and any entity: toggle, levels, scenes, live state and ready-made keys |

## How it fits together

```text
plugins/<id>/        a plugin: plugin.toml, src/, assets/ (icons), tests/, README.md
tests/host/          the plugins running in the real plugin host against mock OBS and Home Assistant
tools/release/       signs packages and builds the plugin index (used by CI)
scripts/             package-plugin.sh (build + pack one plugin), release.sh (CI)
```

Plugins are WebAssembly components built with the SDK (`ulanzi-plugin`) from the app's repo. The
workspace pins the app commit they are built and tested against (`[workspace.dependencies]` in
`Cargo.toml`); move the pin when a plugin needs something newer. How to write a plugin, the manifest
and the contract: the app's [plugin guide](https://github.com/Tyaaa-aa/Ulanzi-Deck-Linux/blob/main/docs/plugins.md).

## Working on a plugin

```bash
cargo test --workspace                       # unit tests, then each plugin in the real host
scripts/package-plugin.sh plugins/ha         # dist/plugins/ha-<version>.ulanziplugin
ulanzictl plugin install dist/plugins/ha-<version>.ulanziplugin   # unsigned: needs Developer Mode
```

Set `contract` and `min_app` in `plugin.toml` to what the plugin uses: an app older than `min_app`
refuses it and says which version it needs.

## Releasing

There is nothing to do by hand. Raise `version` in a plugin's `plugin.toml` and merge to `main`:

1. CI runs the tests.
2. For every plugin whose version has no release yet, it builds the package, signs it and publishes
   the release `<id>-v<version>` with `<id>-<version>.ulanziplugin` and its `.minisig`.
3. It rebuilds `index.json` (the current version of every plugin, with each package's SHA-256), signs
   it and puts both on the `index` release. The app reads that index.

Pull requests are tested but never signed.

## Signing

Packages and the index are signed with [minisign](https://jedisct1.github.io/minisign/) (Ed25519). The
app has the public key built in:

```text
RWT2/HtBKUuAWImA/92lW+GmNaz2nfsNJrxbGrfVmSj/sPyQlngs0pCf
```

CI signs with the secret key and its password, kept as the repository's Actions secrets
`MINISIGN_KEY` and `MINISIGN_PASSWORD`. Workflows can use them; nobody can read them back. Check a
download yourself:

```bash
minisign -Vm ha-1.2.0.ulanziplugin -P RWT2/HtBKUuAWImA/92lW+GmNaz2nfsNJrxbGrfVmSj/sPyQlngs0pCf
```

## Adding your plugin

Open a pull request with `plugins/<id>/` (source, `plugin.toml`, icons in `assets/`, tests and a
README). CI builds and tests it; once merged it is signed and listed like the others.

## Licence

MIT. Icon credits are in each plugin's `assets/CREDITS.md`.
