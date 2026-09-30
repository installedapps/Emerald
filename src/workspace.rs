use anyhow::{bail, Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Filesystem boundary for an Emerald workspace.
///
/// Paths returned by this type are canonical paths beneath the workspace root.
/// Names passed to mutating operations are single file names, never paths.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    #[tracing::instrument(level = "info", skip_all, fields(root = %root.display()), err)]
    pub fn open(root: &Path) -> Result<Self> {
        fs::create_dir_all(root)?;
        let root = fs::canonicalize(root)?;
        let workspace = Self { root };
        workspace.ensure_welcome_file()?;
        Ok(workspace)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    #[tracing::instrument(level = "debug", skip(self), err)]
    pub fn list_asciidoc_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    tracing::warn!(error = %error, "skipping an unreadable workspace entry");
                    continue;
                }
            };
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    tracing::warn!(
                        path = %entry.path().display(),
                        error = %error,
                        "could not inspect workspace entry"
                    );
                    continue;
                }
            };
            if file_type.is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|extension| extension.to_str())
                    == Some("adoc")
            {
                files.push(entry.path());
            }
        }
        files.sort();
        tracing::debug!(count = files.len(), "listed AsciiDoc files");
        Ok(files)
    }

    pub fn resolve_existing(&self, file: impl AsRef<Path>) -> Result<PathBuf> {
        let path = self.input_path(file.as_ref());
        let canonical = fs::canonicalize(&path)
            .with_context(|| format!("cannot resolve file: {}", path.display()))?;
        self.ensure_contained(&canonical)?;
        if !canonical.is_file() {
            bail!("not a file: {}", canonical.display());
        }
        if canonical
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("adoc")
        {
            bail!("only .adoc files are supported: {}", canonical.display());
        }
        Ok(canonical)
    }

    #[tracing::instrument(level = "debug", skip(self, file), fields(file = %file.as_ref().display()), err)]
    pub fn read_file(&self, file: impl AsRef<Path>) -> Result<String> {
        let path = self.resolve_existing(file)?;
        fs::read_to_string(&path)
            .with_context(|| format!("cannot read AsciiDoc file: {}", path.display()))
    }

    #[tracing::instrument(level = "info", skip(self), fields(name), err)]
    pub fn create_file(&self, name: &str) -> Result<PathBuf> {
        let base = valid_file_name(name)?;
        let mut suffix = 1;
        loop {
            let candidate_name = if suffix == 1 {
                format!("{base}.adoc")
            } else {
                format!("{base}-{suffix}.adoc")
            };
            let candidate = self.root.join(candidate_name);
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(_) => return Ok(candidate),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    suffix += 1;
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    #[tracing::instrument(level = "info", skip(self, contents, file), fields(file = %file.as_ref().display(), bytes = contents.len()), err)]
    pub fn write_file(&self, file: impl AsRef<Path>, contents: &str) -> Result<()> {
        let path = self.resolve_existing(file)?;
        let temporary = path.with_extension("adoc.tmp");
        {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .with_context(|| format!("cannot stage file write: {}", path.display()))?;
            file.write_all(contents.as_bytes())
                .with_context(|| format!("cannot write staged file: {}", path.display()))?;
            file.sync_all()
                .with_context(|| format!("cannot sync staged file: {}", path.display()))?;
        }
        fs::rename(&temporary, &path)
            .with_context(|| format!("cannot commit file write: {}", path.display()))?;
        Ok(())
    }

    #[tracing::instrument(level = "info", skip(self, file), fields(file = %file.as_ref().display()), err)]
    pub fn remove_file(&self, file: impl AsRef<Path>) -> Result<()> {
        let path = self.resolve_existing(file)?;
        fs::remove_file(&path)
            .with_context(|| format!("cannot remove AsciiDoc file: {}", path.display()))?;
        Ok(())
    }

    #[tracing::instrument(level = "info", skip(self, file), fields(file = %file.as_ref().display(), name), err)]
    pub fn rename_file(&self, file: impl AsRef<Path>, name: &str) -> Result<PathBuf> {
        let source = self.resolve_existing(file)?;
        let destination = self.root.join(valid_destination_name(name)?);
        if destination == source {
            return Ok(source);
        }
        if destination
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("adoc")
        {
            bail!("only .adoc files are supported");
        }
        if destination.exists() {
            bail!("a file with that name already exists");
        }
        self.ensure_contained(&destination)?;
        fs::rename(source, &destination)?;
        Ok(destination)
    }

    fn ensure_welcome_file(&self) -> Result<()> {
        let welcome = self.root.join("welcome.adoc");
        match fs::symlink_metadata(&welcome) {
            Ok(metadata) if metadata.file_type().is_file() => Ok(()),
            Ok(_) => bail!("workspace welcome.adoc is not a regular file"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(welcome)?;
                file.write_all(b"= Emerald\n\nStart writing AsciiDoc here.\n")?;
                Ok(())
            }
            Err(error) => Err(error.into()),
        }
    }

    fn input_path(&self, file: &Path) -> PathBuf {
        if file.is_absolute() {
            file.to_path_buf()
        } else {
            self.root.join(file)
        }
    }

    fn ensure_contained(&self, path: &Path) -> Result<()> {
        if path.strip_prefix(&self.root).is_err() {
            bail!("file is outside the workspace: {}", path.display());
        }
        Ok(())
    }
}

fn valid_file_name(name: &str) -> Result<String> {
    let base = name.trim().trim_end_matches(".adoc").replace(' ', "-");
    if base.is_empty()
        || base == "."
        || base == ".."
        || base.contains(['/', '\\'])
        || Path::new(&base).components().count() != 1
    {
        bail!("invalid file name");
    }
    Ok(base)
}

fn valid_destination_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\'])
        || Path::new(name).components().count() != 1
    {
        bail!("invalid file name");
    }
    Ok(if Path::new(name).extension().is_some() {
        name.to_string()
    } else {
        format!("{name}.adoc")
    })
}

#[cfg(test)]
#[path = "tests/workspace.rs"]
mod tests;
