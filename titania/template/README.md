# Titania workspace template

This `titania/template` cargo-generate source creates a strict Rust workspace configured for the Titania Check v1 contract. It installs the strict-ai policy files, Rust lint baseline, cargo-deny config, rustfmt settings, and Moon-managed toolchain pin used by Titania.

## Generate

```bash
cargo generate titania/template --name my-workspace
```

After generation, keep policy exceptions in `.titania/profiles/strict-ai/exceptions.toml` and run the repository's Titania/Moon gates before landing changes.
