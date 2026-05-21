# Velvet Ballastics Tooling

Standalone deterministic tooling for the sibling `velvet-ballistics` source repository.

From `../velvet-ballistics`, run commands through the Cargo alias:

```bash
cargo xtask contracts --check
```

From this project, run the tooling gates directly:

```bash
cargo fmt --all -- --check
cargo clippy --locked --lib --bins --examples --all-features -- -D warnings
cargo test --locked --all-features
```
