use std::borrow::Cow;
use std::ops::Range;

pub(crate) fn sanitize_file_name<'a>(
    file_name: impl Into<Cow<'a, str>>,
    replacement: &'a str,
) -> Cow<'a, str> {
    let mut file_name = file_name.into();

    if file_name.is_empty() || file_name.chars().all(char::is_whitespace) {
        return replacement.into();
    }

    replace_consecutive(&mut file_name, is_disallowed_char, replacement);

    if is_reserved_file_name(&file_name) {
        file_name.to_mut().insert_str(0, replacement);
    }

    file_name
}

#[cfg(windows)]
fn is_disallowed_char(c: char) -> bool {
    // Source: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file#naming-conventions
    // Instead of just disallowing ASCII control characters, I opted to disallow all control characters.
    // Disallowing colon (:) is really important as allowing it could allow writing to NTFS alternate data streams.
    matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control()
}

#[cfg(target_os = "macos")]
fn is_disallowed_char(c: char) -> bool {
    // See: https://superuser.com/a/326627
    // macOS only disallows / but files containing colons (:) cannot be created in Finder.
    // Disallowing control characters just seems like a good idea to me.
    matches!(c, '/' | ':') || c.is_control()
}

#[cfg(not(any(windows, target_os = "macos")))]
fn is_disallowed_char(c: char) -> bool {
    // Disallowing control characters just seems like a good idea to me.
    c == '/' || c.is_control()
}

// Source: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file#naming-conventions
#[cfg(windows)]
fn is_reserved_file_name(file_name: &str) -> bool {
    macro_rules! matches_ignore_case {
        ($target:ident, $($p:literal)|+) => {
            $($target.eq_ignore_ascii_case($p))||+
        };
    }

    matches_ignore_case!(
        file_name,
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM0"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT0"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

#[cfg(not(windows))]
fn is_reserved_file_name(_file_name: &str) -> bool {
    false
}

// Replaces the given pattern with the replacement.
// Consecutive matches are replaced once.
fn replace_consecutive<'a>(
    haystack: &mut Cow<'a, str>,
    mut pattern: impl FnMut(char) -> bool,
    replacement: &'a str,
) {
    let mut index = 0;
    // TODO: use let chains once stabilized
    while index < haystack.len() {
        if let Some(range) = next_match(&haystack[index..], &mut pattern) {
            let absolute_range = (index + range.start)..(index + range.end);
            haystack.to_mut().replace_range(absolute_range, replacement);
            index += range.end;
        } else {
            break;
        }
    }
}

fn next_match(haystack: &str, mut pattern: impl FnMut(char) -> bool) -> Option<Range<usize>> {
    let start = haystack.find(&mut pattern)?;

    let len: usize = haystack[start..]
        .chars()
        .take_while(|c| pattern(*c))
        .map(|c| c.len_utf8())
        .sum();

    Some(start..(start + len))
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPLACEMENT: &str = "_";

    #[test]
    fn replaces_disallowed_chars() {
        assert_eq!(
            sanitize_file_name("/foo/bar/baz", REPLACEMENT),
            "_foo_bar_baz"
        );
        assert_eq!(
            sanitize_file_name("foo/\0/\0/\0/bar", REPLACEMENT),
            "foo_bar"
        );
        assert_eq!(sanitize_file_name("//////////////", REPLACEMENT), "_");
    }

    #[test]
    fn ensures_filename_is_not_empty() {
        assert_eq!(sanitize_file_name("", REPLACEMENT), "_");
        assert_eq!(sanitize_file_name("   ", REPLACEMENT), "_");
        assert_eq!(sanitize_file_name("\t\r\n ", REPLACEMENT), "_");
    }

    #[cfg(windows)]
    #[test]
    fn prefixes_reserved_file_names_with_replacement() {
        assert_eq!(sanitize_file_name("NUL", REPLACEMENT), "_NUL");
        assert_eq!(sanitize_file_name("aux", REPLACEMENT), "_aux");
    }
}
