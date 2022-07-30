use std::{ffi::OsString, io::Write};

use unicode_xid::UnicodeXID;

/// Type of completion attempted that caused a completion function to be called
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompType {
    /// Normal completion
    Normal,
    /// List completions after successive tabs
    Successive,
    /// List alternatives on partial word completion
    Alternatives,
    /// List completions if the word is not unmodified
    Unmodified,
    /// Menu completion
    Menu,
}

impl clap::ArgEnum for CompType {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Normal,
            Self::Successive,
            Self::Alternatives,
            Self::Unmodified,
            Self::Menu,
        ]
    }

    fn to_possible_value<'a>(&self) -> ::std::option::Option<clap::PossibleValue<'a>> {
        match self {
            Self::Normal => {
                let value = "9";
                debug_assert_eq!(b'\t'.to_string(), value);
                Some(
                    clap::PossibleValue::new(value)
                        .alias("normal")
                        .help("Normal completion"),
                )
            }
            Self::Successive => {
                let value = "63";
                debug_assert_eq!(b'?'.to_string(), value);
                Some(
                    clap::PossibleValue::new(value)
                        .alias("successive")
                        .help("List completions after successive tabs"),
                )
            }
            Self::Alternatives => {
                let value = "33";
                debug_assert_eq!(b'!'.to_string(), value);
                Some(
                    clap::PossibleValue::new(value)
                        .alias("alternatives")
                        .help("List alternatives on partial word completion"),
                )
            }
            Self::Unmodified => {
                let value = "64";
                debug_assert_eq!(b'@'.to_string(), value);
                Some(
                    clap::PossibleValue::new(value)
                        .alias("unmodified")
                        .help("List completions if the word is not unmodified"),
                )
            }
            Self::Menu => {
                let value = "37";
                debug_assert_eq!(b'%'.to_string(), value);
                Some(
                    clap::PossibleValue::new(value)
                        .alias("menu")
                        .help("Menu completion"),
                )
            }
        }
    }
}

impl Default for CompType {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Clone, Debug, clap::Args)]
#[allow(missing_docs)]
pub struct CompleteArgs {
    #[clap(
        long,
        required = true,
        value_name = "COMP_CWORD",
        hide_short_help = true,
        value_parser
    )]
    index: Option<usize>,

    #[clap(long, hide_short_help = true, value_parser)]
    ifs: Option<String>,

    #[clap(long = "type", required = true, hide_short_help = true, value_parser)]
    comp_type: Option<CompType>,

    #[clap(long, hide_short_help = true, action)]
    space: bool,

    #[clap(long, conflicts_with = "space", hide_short_help = true, action)]
    no_space: bool,

    #[clap(raw = true, hide_short_help = true, value_parser)]
    comp_words: Vec<OsString>,
}

/// The recommended file name for the registration code
pub fn file_name(name: &str) -> String {
    format!("{}.bash", name)
}

/// Define the completion behavior
pub enum Behavior {
    /// Bare bones behavior
    Minimal,
    /// Fallback to readline behavior when no matches are generated
    Readline,
    /// Customize bash's completion behavior
    Custom(String),
}

impl Default for Behavior {
    fn default() -> Self {
        Self::Readline
    }
}

/// Generate code to register the dynamic completion
pub fn register(
    name: &str,
    executables: impl IntoIterator<Item = impl AsRef<str>>,
    completer: &str,
    behavior: &Behavior,
    buf: &mut dyn Write,
) -> Result<(), std::io::Error> {
    let escaped_name = name.replace("-", "_");
    debug_assert!(
        escaped_name.chars().all(|c| c.is_xid_continue()),
        "`name` must be an identifier, got `{}`",
        escaped_name
    );
    let mut upper_name = escaped_name.clone();
    upper_name.make_ascii_uppercase();

    let executables = executables
        .into_iter()
        .map(|s| shlex::quote(s.as_ref()).into_owned())
        .collect::<Vec<_>>()
        .join(" ");

    let options = match behavior {
        Behavior::Minimal => "-o nospace -o bashdefault",
        Behavior::Readline => "-o nospace -o default -o bashdefault",
        Behavior::Custom(c) => c.as_str(),
    };

    let completer = shlex::quote(completer);

    let script = r#"
__clap_complete_NAME_debug() {
    local file="$BASH_COMP_DEBUG_FILE"
    if [[ -n ${file} ]]; then
        echo "$*" >> "${file}"
    fi
}

_clap_complete_NAME() {
    local compCmd
    local IFS=$'\013'
    local SUPPRESS_SPACE=0
    if compopt +o nospace 2> /dev/null; then
        SUPPRESS_SPACE=1
    fi
    if [[ ${SUPPRESS_SPACE} == 1 ]]; then
        SPACE_ARG="--no-space"
    else
        SPACE_ARG="--space"
    fi

    compCmd="COMPLETER complete --index ${COMP_CWORD} --type ${COMP_TYPE} ${SPACE_ARG} --ifs=$IFS -- ${COMP_WORDS[@]}"

    __clap_complete_NAME_debug "Calling completion command: eval ${compCmd}"

    COMPREPLY=( $("COMPLETER" complete --index ${COMP_CWORD} --type ${COMP_TYPE} ${SPACE_ARG} --ifs="$IFS" -- "${COMP_WORDS[@]}") )
    __clap_complete_NAME_debug "Completion command output: ${COMPREPLY}"

    if [[ $? != 0 ]]; then
        unset COMPREPLY
    elif [[ $SUPPRESS_SPACE == 1 ]] && [[ "${COMPREPLY-}" =~ [=/:]$ ]]; then
        compopt -o nospace
    fi
}

complete OPTIONS -F _clap_complete_NAME EXECUTABLES
"#
    .replace("NAME", &escaped_name)
    .replace("EXECUTABLES", &executables)
    .replace("OPTIONS", options)
    .replace("COMPLETER", &completer)
    .replace("UPPER", &upper_name);

    writeln!(buf, "{}", script)?;
    Ok(())
}

/// Process the completion request for bash
pub fn complete(cmd: &mut clap::Command, args: &CompleteArgs) -> clap::Result<()> {
    let index = args.index.unwrap_or_default();
    let _comp_type = args.comp_type.unwrap_or_default();
    let _space = match (args.space, args.no_space) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        (true, true) => {
            unreachable!("`--space` and `--no-space` set, clap should prevent this")
        }
        (false, false) => None,
    }
    .unwrap();

    let current_dir = std::env::current_dir().ok();
    let completions =
        super::complete::get(cmd, args.comp_words.clone(), index, current_dir.as_deref())?;

    let mut buf = Vec::new();
    for (i, completion) in completions.iter().enumerate() {
        if i != 0 {
            write!(&mut buf, "{}", args.ifs.as_deref().unwrap_or("\n"))?;
        }
        write!(&mut buf, "{}", completion.to_string_lossy())?;
    }
    std::io::stdout().write(&buf)?;

    Ok(())
}
