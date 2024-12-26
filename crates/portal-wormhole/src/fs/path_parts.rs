use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub(super) struct PathParts<'a> {
    path: &'a Path,
    stem: &'a OsStr,
    extension: Option<&'a OsStr>,
}

impl<'a> TryFrom<&'a Path> for PathParts<'a> {
    type Error = ();

    fn try_from(path: &'a Path) -> Result<Self, Self::Error> {
        let stem = path.file_stem().ok_or(())?;
        let extension = path.extension();
        Ok(PathParts {
            path,
            stem,
            extension,
        })
    }
}

impl PathParts<'_> {
    pub(crate) fn to_path_with_counter(&self, counter: u64) -> PathBuf {
        if counter == 0 {
            self.path.to_owned()
        } else {
            let mut file_name = self.stem.to_owned();
            file_name.push(format!(" ({counter})"));

            if let Some(extension) = self.extension {
                file_name.push(".");
                file_name.push(extension);
            }

            self.path.with_file_name(file_name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_path_with_counter() {
        for (expected, input) in [
            ("foo (1).bar", "foo.bar"),
            ("foo (1)", "foo"),
            (".bar (1)", ".bar"),
        ] {
            assert_eq!(
                expected,
                PathParts::try_from(Path::new(input))
                    .expect("input to be a valid path")
                    .to_path_with_counter(1)
                    .to_str()
                    .expect("path to be valid unicode")
            );
        }
    }
}
