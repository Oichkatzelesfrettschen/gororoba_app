# Desktop Packaging Lane

## Scope

Reproducible binary packaging for Linux/BSD/Windows/macOS targets using Rust-only
workspace automation (`xtask`).

## Commands

Build/package matrix:

```bash
scripts/package_desktop_matrix.sh
```

Enable cross-target attempts explicitly:

```bash
GOROROBA_ENABLE_CROSS=1 GOROROBA_CROSS_TOOLCHAINS_READY=1 scripts/package_desktop_matrix.sh
```

Generate manifest/checksums from existing dist tree:

```bash
cargo run -p xtask -- desktop-manifest --dist-dir dist/desktop
```

## Output structure

- `dist/desktop/<target>/<binary>`
- `dist/desktop/manifest.json`
- `dist/desktop/checksums.txt`

Default packaged binaries:

1. `gororoba_studio_web`
2. `physics_sandbox`
3. `synthesis_arena`

## Notes

1. Default behavior packages host target only; cross-target packaging is opt-in.
2. Cross-target compilation may require target toolchains/linkers and installed rust targets.
3. `GOROROBA_ENABLE_CROSS=1` enables matrix selection from `GOROROBA_PACKAGE_TARGETS`.
4. `GOROROBA_CROSS_TOOLCHAINS_READY=1` confirms cross linker/toolchain setup.
5. Missing or unready targets are skipped during preflight with explicit messages.
6. `GOROROBA_PACKAGE_ALLOW_MISSING=1` keeps permissive packaging mode in `xtask`.
