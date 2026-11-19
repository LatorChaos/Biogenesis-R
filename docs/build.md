# Build and run

Build on Linux x86_64 or ARM64. The Raspberry Pi setup uses Raspberry Pi OS 64-bit Desktop.

Install Godot 4.7.2 stable, Python 3.11 or newer, Rust 1.94 or newer, and a native C linker. Put `godot`, `cargo`, `rustc`, and `rustdoc` on your PATH.

Run these commands from the repository root to fetch the locked dependencies and build the release adapter.

```sh
cargo fetch --locked
python3 tools/build.py build --release --timeout 3600 --cache-source "${CARGO_HOME:-$HOME/.cargo}"
```

The helper uses a private offline Cargo cache. It puts the native adapter in `bin/` for your architecture.

Use the build helper for the first import and launch. It opens a project-local profile and gives each step up to one hour.

```sh
python3 tools/build.py run --release --timeout 3600 --cache-source "${CARGO_HOME:-$HOME/.cargo}"
```

If the Godot executable has a different name, pass `--godot` followed by its path to the build helper.

For later sessions, run Godot directly from the repository root with the same profile.

```sh
XDG_DATA_HOME="$PWD/.workflow-local/runtime/data" godot --path .
```

## Fullscreen Cinema on a Raspberry Pi

Leave Cinema and Continuous display enabled. Touch the world to bring back the controls. Use **Tools > Parameters > Cinema** to adjust the camera pace and when records appear.

The app keyboard is enabled by default. To try a desktop keyboard instead, create an empty `desktop-keyboard` file in the Godot user-data folder before launching. With the profile below, that file is `.workflow-local/runtime/data/godot/app_userdata/Biogenesis-R/desktop-keyboard`. If the desktop keyboard cannot type, cancel the dialog and choose **Tools > World > Use app keyboard** to restore the app keyboard.

Enable automatic backup in **Tools > Parameters** and choose **Managed recovery**. Create and save a world. Check the latest backup time in **Tools** before relying on automatic resume.

Quit the running game before starting another instance. To resume later, point Godot to the same data directory.

```sh
XDG_DATA_HOME="$PWD/.workflow-local/runtime/data" \
  godot --path . --display-driver wayland --rendering-method gl_compatibility --audio-driver Dummy --max-fps 30 -- --safe-graphics --resume-managed
```

If recovery needs your attention, the game starts paused. Use **Tools > Game > Open** to choose a saved world.

## Tests

[Fixture provenance and path redaction](fixture-provenance.md) explains the retained reference records.

Run the Rust tests and the Godot boundary checks from the repository root. Networking tests use local loopback sockets.

```sh
python3 tools/build.py test --release --timeout 3600 --cache-source "${CARGO_HOME:-$HOME/.cargo}"
python3 tools/build.py boundary --release --timeout 3600 --cache-source "${CARGO_HOME:-$HOME/.cargo}"
```
