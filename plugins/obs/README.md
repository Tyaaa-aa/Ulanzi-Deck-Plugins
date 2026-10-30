# OBS Studio plugin

Controls OBS Studio from Ulanzi Deck and shows its state on keys and on the wide screen. Requires
OBS 28 or later (obs-websocket 5).

The plugin only connects to OBS on the local machine (`network = ["127.0.0.1", "localhost"]`). While
OBS is not running, it retries with backoff (1 s, doubling to 10 s), and its status describes the
problem. The retry interval resets to 1 s only after OBS has returned its version and scene list, so
an OBS instance that accepts the connection but rejects those requests is retried at most every
10 seconds.

Profiles from Ulanzi Deck 2.0 are converted to this plugin automatically. The 2.0 label variables
`{obs_scene}`, `{obs_rec_time}` and `{obs_stream_time}` continue to work.

## Installation

Install the plugin from **Plugins → Discover** in Ulanzi Deck, or with `ulanzictl plugin get obs`.

In OBS, open **Tools → WebSocket Server Settings** and enable the WebSocket server. **Show Connect
Info** shows the port and password.

### Building from source

```sh
cargo build --release --target wasm32-wasip2 -p ulanzi-plugin-obs
mkdir -p ~/obs-plugin
cp plugins/obs/plugin.toml ~/obs-plugin/
cp target/wasm32-wasip2/release/ulanzi_plugin_obs.wasm ~/obs-plugin/plugin.wasm
ulanzictl plugin dev-mode on
ulanzictl plugin install ~/obs-plugin
```

To build a package instead, and install it with **Plugins → Install Plugin…** (Developer Mode on)
or `ulanzictl plugin install`:

```sh
scripts/package-plugin.sh plugins/obs     # writes dist/plugins/obs-<version>.ulanziplugin
```

## Settings

In Ulanzi Deck, open **Plugins → Installed → OBS Studio** and enter the address, port and
password. The password is stored in the plugin's secret store.

### Configuring without the app

Edit `~/.config/ulanzi-deck/settings.toml`, then run `systemctl --user reload ulanzi-deckd`:

```toml
[plugins.obs]
enabled = true

[plugins.obs.settings]
host = "localhost"      # or "127.0.0.1"; anything else is refused
port = 4455
password = "..."        # only for the first start, see below
```

On first start, the password is saved to the system keyring. Then delete the `password` line and
reload again; the plugin logs a warning on every start while the line is present. Until the line is
removed (deleting it manually or entering a password in the app both remove it), the password is
stored in plain text in `settings.toml` and its local backups. D-Bus `GetSettings` shows it as
`••••`, and settings exports omit it. The `forget_password` action and `ulanzictl plugin remove obs`
both delete the saved password.

## Actions

In `profile.toml`, use `type = "plugin"`, `plugin = "obs"`, `action = "<id>"` and `params = { ... }`.

| Action | Params |
|---|---|
| `set_scene` | `scene`; `preview = true` sets the preview scene (studio mode) |
| `toggle_scenes` | `a`, `b`: switch to `b` when `a` is live, otherwise to `a` |
| `toggle_source` | `scene`, `source`: show or hide |
| `toggle_record`, `start_record`, `stop_record`, `pause_record` | none (`pause_record` pauses or resumes) |
| `toggle_stream`, `start_stream`, `stop_stream` | none |
| `toggle_replay`, `save_replay` | none |
| `toggle_virtual_cam` | none |
| `toggle_mute` | `input` |
| `input_volume` | `input`, `op` = `up` / `down` / `mute` (mutes or unmutes) / `set`; `percent` (0-100, for `set`); `step` in dB (1-20, default 2); `up` from 0 % starts at −100 dB |
| `toggle_filter` | `source`, `filter` |
| `studio_transition` | none |
| `set_collection` | `name` |
| `set_profile` | `name` |
| `screenshot` | none |
| `reconnect` | none |
| `forget_password` | none |

```toml
[pages.keys.7.look]
label = { text = "{obs.rec_time}", visible = true }

[pages.keys.7.tap]
type = "plugin"
plugin = "obs"
action = "toggle_record"

[pages.keys.8.look]
label = { text = "{obs.scene}", visible = true }

[pages.keys.8.tap]
type = "plugin"
plugin = "obs"
action = "set_scene"
params = { scene = "Gameplay" }
```

## Pickers

Parameters that refer to an OBS object are selected from a list in the app. Lists are built from
data the plugin already holds, without querying OBS when a list opens, so OBS must be connected.
Otherwise the picker reports "Not connected to OBS" and the name can be entered manually.

| Picker | Used by | Entries |
|---|---|---|
| `scenes` | `set_scene.scene`, `toggle_scenes.a` / `b`, `toggle_source.scene` | scenes; "Live" for the program scene, "Preview" for the preview scene |
| `audio_inputs` | `toggle_mute.input`, `input_volume.input` | inputs that have audio; "Muted" when muted |
| `scene_sources` | `toggle_source.source` | each source in each scene, grouped by scene |
| `sources` | `toggle_filter.source` | every source that appears in a scene |
| `filters` | `toggle_filter.filter` | filters, grouped by source; "On" or "Off" |
| `collections` | `set_collection.name` | scene collections; "Current" for the active one |
| `profiles` | `set_profile.name` | OBS profiles; "Current" for the active one |

Search matches any part of a name, case-insensitively. Filter lists are fetched for at most 200
sources (25 requests at a time) and refresh when OBS reports a change. They do not use state keys.

## Live state

These values are available in key labels as `{obs.<key>}`. Boolean values are `yes` or `no`, and
times use `mm:ss` or `h:mm:ss`. While OBS is disconnected, boolean values are `no` and names, times,
statistics, `mute.…`, `visible.…` and `last_error` are empty.

| Key | Value |
|---|---|
| `connected` | `yes` / `no` |
| `scene`, `preview` | live scene; preview scene (studio mode only) |
| `studio_mode` | `yes` / `no` |
| `scene_count` | number of scenes |
| `recording`, `rec_paused`, `streaming`, `replay`, `vcam` | `yes` / `no` |
| `rec_time`, `stream_time` | time running; empty when the output is off |
| `fps`, `cpu`, `dropped` | from OBS stats, refreshed every 2 s; `dropped` is a percentage like `0.4%` |
| `last_error` | OBS's refusal of the last action, empty after the next success |
| `mute.<input>` | `yes` / `no` for each audio input |
| `visible.<scene>/<source>` | `yes` / `no` for each source in a scene |

`visible.<scene>/<source>` is ambiguous when a scene or source name contains `/` (`a/b` with `c`
and `a` with `b/c` produce the same key). A source that appears twice in one scene shares one key,
which is `no` if any instance is hidden.

### Limits

- At most 200 `mute.…` and `visible.…` keys at a time.
- The host allows each plugin 256 distinct state keys for the lifetime of the process, and keys
  cannot be removed. Every scene, source or input name the plugin publishes counts toward this, so
  frequent renames or scene collection switches use up keys even though old values become empty.
  When the limit is reached, new names get no value and the plugin logs a warning. Disabling and
  re-enabling the plugin, or restarting `ulanzi-deckd`, resets the count.
- Keys longer than 256 bytes are skipped, and values longer than 1 KiB are truncated.

## Known limitations

| Limitation | Reason |
|---|---|
| No automatic detection of the OBS port and password, and the WebSocket server cannot be enabled from Ulanzi Deck | Plugins have no file system access |
| "Not installed", "not running", "server disabled" and "wrong port" are reported as one message | The plugin only sees that the connection failed |
| Connecting can take up to 10 seconds after OBS starts | The plugin cannot watch processes and relies on retries |
| OBS on another computer is not supported | The network permission is limited to the local machine |
| Profiles are not updated when scenes or inputs are renamed | Plugins cannot edit profiles |
| Errors returned by OBS are not shown as the key's error | Actions return before OBS responds; see `{obs.last_error}` and the log |
| A wrong password is reported as "OBS closed the connection during sign-in" | The host does not pass the WebSocket close code to plugins |

## Tests

`cargo test -p ulanzi-plugin-obs` runs the plugin natively with `Harness`
([tests/plugin.rs](tests/plugin.rs)). `cargo test -p host-tests --test obs_plugin` runs the built
WebAssembly component in the real plugin host against a mock OBS server.
