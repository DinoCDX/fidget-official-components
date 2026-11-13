# Fidget official components

Components published by DinoCDX for the Fidget app. Each component lives in `components/<id>/`:

- `component.toml`: manifest. `repository` must be this repo.
- `layout.toml`: the settings layout that Fidget builds its settings screen from.
- `crates/`: the Rust code, if the component has any.

## Releases

The workflow in `.github/workflows/release.yml` builds and publishes components:

- Manual run (Actions > Components release): publishes `<id>-debug-N` as a prerelease, where N is the run number.
- Push a tag `<id>-v<version>` (for example `power-search-v0.1.0`): publishes a stable release. The tag must match the version in the component's `component.toml`.

Each release has one zip per platform (`<tag>-windows.zip`, `<tag>-linux.zip`) and a `.sha256` file. The zip holds `component.toml`, `layout.toml`, and the executable.

## Adding a component

1. Create `components/<id>/` with `component.toml` and `layout.toml`.
2. If it's Rust, add its crates under `components/<id>/crates/`. The workspace picks them up automatically.
3. Add a matrix entry for it in `release.yml`, and set its `cargo_package` and `binary`.

## Build

    cargo test --workspace
    cargo build -p launcher-run
