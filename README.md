# hodr

`hodr` is a small CLI for adding repo files and directories to `.gitignore` from an interactive fuzzy menu.

It lists untracked files with Git, filters out anything already ignored by `.gitignore`, and opens a `skim` picker so you can select one or more entries to append to the repository's `.gitignore`.

## Features

- Fuzzy-select files from an interactive terminal menu.
- Select multiple entries in one run.
- Select parent directories as well as individual files.
- Excludes files already ignored by Git.
- Appends only new `.gitignore` entries and skips duplicates.
- Warns instead of failing when run outside a Git repository.

## Requirements

- Git
- Rust toolchain

## Install

From a local checkout:

```sh
cargo install --path .
```

Or run without installing:

```sh
cargo run
```

## Usage

Run `hodr` from inside a Git repository:

```sh
hodr
```

Use the fuzzy menu to search, select files or directories, and confirm your selection. `hodr` appends the selected entries to `.gitignore` at the repository root.

If no untracked files are available, it exits without changing anything.

## Development

Run the test suite:

```sh
cargo test
```

## License

GPL-3.0. See [LICENSE](LICENSE).
