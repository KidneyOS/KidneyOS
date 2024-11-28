// Reference: https://man7.org/linux/man-pages/man1/ls.1.html
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Display;

#[allow(dead_code)]
pub enum Format {
    /// Default
    Columns,

    /// -1
    ///
    /// list one file per line
    OneLine,

    /// -l
    ///
    /// use a long listing format
    Long,

    /// -m
    ///
    /// fill width with a comma separated list of entries
    Commas,

    /// -x
    ///
    /// list entries by lines instead of by columns
    Across,
}

impl Display for Format {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Format::Columns => write!(f, "Columns"),
            Format::OneLine => write!(f, "OneLine"),
            Format::Long => write!(f, "Long"),
            Format::Commas => write!(f, "Commas"),
            Format::Across => write!(f, "Across"),
        }
    }
}

#[allow(dead_code)]
pub enum Files {
    /// Default
    Normal,

    /// -A, --almost-all
    ///
    /// do not list implied . and ..
    AlmostAll,

    /// -a, --all
    ///
    /// do not ignore entries starting with .
    All,
}

impl Display for Files {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Files::Normal => write!(f, "Normal"),
            Files::AlmostAll => write!(f, "AlmostAll"),
            Files::All => write!(f, "All"),
        }
    }
}

#[allow(dead_code)]
pub struct LsConfig {
    pub format: Format,
    files: Files,
}

impl Display for LsConfig {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "LsConfig {{ format: {}, files: {} }}",
            self.format, self.files
        )
    }
}

#[allow(dead_code)] // ??? WHY???
impl LsConfig {
    pub fn new() -> LsConfig {
        LsConfig {
            format: Format::Columns,
            files: Files::Normal,
        }
    }

    pub fn from_args(args: Vec<&str>) -> LsConfig {
        let mut format = Format::Columns;
        let mut files = Files::Normal;

        let args = preprocess_args(args);

        for arg in args {
            match arg.as_str() {
                "-1" => format = Format::OneLine,
                "-l" => format = Format::Long,
                "-m" => format = Format::Commas,
                "-x" => format = Format::Across,
                "-A" | "--almost-all" => files = Files::AlmostAll,
                "-a" | "--all" => files = Files::All,
                _ => {}
            }
        }

        LsConfig { format, files }
    }
}

fn preprocess_args(args: Vec<&str>) -> Vec<String> {
    let mut new_args: Vec<String> = Vec::new();

    for arg in args {
        if arg.starts_with('-') {
            if arg.len() > 2 {
                // Multiple flags in one argument, e.g. "-la"
                for c in arg.chars().skip(1) {
                    new_args.push(format!("-{}", c).to_string());
                }
            } else {
                // Single flag, e.g. "-l"
                new_args.push(arg.to_string());
            }
        } else {
            // Not a flag, e.g. "file.txt"
            // Currently this is not supported
            // new_args.push(arg.to_string());
        }
    }

    new_args
}
