use skim::prelude::*;
use std::collections::BTreeSet;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() {
    if let Err(err) = run() {
        eprintln!("hodr: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let Some(repo_root) = repo_root()? else {
        eprintln!("warning: hodr must be run inside a git repository");
        return Ok(());
    };

    let candidates = menu_candidates(&repo_root)?;
    if candidates.is_empty() {
        eprintln!("hodr: no untracked files to ignore");
        return Ok(());
    }

    let selected = select_with_skim(&candidates)?;
    if selected.is_empty() {
        return Ok(());
    }

    let added = append_unique_gitignore_entries(&repo_root.join(".gitignore"), &selected)?;
    println!(
        "hodr: added {added} entr{}",
        if added == 1 { "y" } else { "ies" }
    );

    Ok(())
}

fn repo_root() -> Result<Option<PathBuf>> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;

    if !output.status.success() {
        return Ok(None);
    }

    let root = String::from_utf8(output.stdout)?;
    Ok(Some(PathBuf::from(root.trim_end())))
}

fn menu_candidates(repo_root: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .current_dir(repo_root)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "failed to list untracked files: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }

    let files = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| String::from_utf8(path.to_vec()))
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(candidates_from_untracked_files(&files))
}

fn candidates_from_untracked_files(files: &[String]) -> Vec<String> {
    let mut candidates = BTreeSet::new();

    for file in files {
        if file == ".gitignore" || file.starts_with(".git/") {
            continue;
        }

        candidates.insert(file.clone());

        let mut path = Path::new(file);
        while let Some(parent) = path.parent() {
            if parent.as_os_str().is_empty() {
                break;
            }

            candidates.insert(format!("{}/", parent.to_string_lossy()));
            path = parent;
        }
    }

    candidates.into_iter().collect()
}

fn select_with_skim(candidates: &[String]) -> Result<Vec<String>> {
    let options = SkimOptionsBuilder::default()
        .height(Some("60%"))
        .multi(true)
        .prompt(Some("ignore> "))
        .build()?;

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();
    for candidate in candidates {
        tx_item.send(Arc::new(candidate.clone()))?;
    }
    drop(tx_item);

    let selected = Skim::run_with(&options, Some(rx_item))
        .filter(|output| !output.is_abort)
        .map(|output| {
            output
                .selected_items
                .iter()
                .map(|item| item.output().to_string())
                .collect()
        })
        .unwrap_or_default();

    Ok(selected)
}

fn append_unique_gitignore_entries(gitignore_path: &Path, selected: &[String]) -> Result<usize> {
    let existing = fs::read_to_string(gitignore_path).unwrap_or_default();
    let existing_lines = existing.lines().collect::<BTreeSet<_>>();
    let additions = selected
        .iter()
        .filter(|entry| !existing_lines.contains(entry.as_str()))
        .collect::<Vec<_>>();

    if additions.is_empty() {
        return Ok(0);
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(gitignore_path)?;

    if !existing.is_empty() && !existing.ends_with('\n') {
        writeln!(file)?;
    }

    for entry in &additions {
        writeln!(file, "{entry}")?;
    }

    Ok(additions.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn candidates_include_untracked_files_and_parent_directories() {
        let files = vec![
            "src/main.rs".to_string(),
            "src/bin/hodr.rs".to_string(),
            "README.md".to_string(),
        ];

        let candidates = candidates_from_untracked_files(&files);

        assert_eq!(
            candidates,
            vec![
                "README.md",
                "src/",
                "src/bin/",
                "src/bin/hodr.rs",
                "src/main.rs",
            ]
        );
    }

    #[test]
    fn candidates_skip_gitignore_and_git_internals() {
        let files = vec![
            ".gitignore".to_string(),
            ".git/hooks/pre-commit".to_string(),
            "notes.txt".to_string(),
        ];

        assert_eq!(candidates_from_untracked_files(&files), vec!["notes.txt"]);
    }

    #[test]
    fn append_unique_entries_creates_gitignore() {
        let dir = temp_dir("create");
        fs::create_dir_all(&dir).unwrap();
        let gitignore = dir.join(".gitignore");

        let added = append_unique_gitignore_entries(
            &gitignore,
            &["target/".to_string(), "scratch.log".to_string()],
        )
        .unwrap();

        assert_eq!(added, 2);
        assert_eq!(
            fs::read_to_string(gitignore).unwrap(),
            "target/\nscratch.log\n"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn append_unique_entries_preserves_content_and_skips_duplicates() {
        let dir = temp_dir("append");
        fs::create_dir_all(&dir).unwrap();
        let gitignore = dir.join(".gitignore");
        fs::write(&gitignore, "target/\nexisting.log").unwrap();

        let added = append_unique_gitignore_entries(
            &gitignore,
            &["target/".to_string(), "new.log".to_string()],
        )
        .unwrap();

        assert_eq!(added, 1);
        assert_eq!(
            fs::read_to_string(gitignore).unwrap(),
            "target/\nexisting.log\nnew.log\n"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("hodr-{label}-{nanos}"))
    }
}
