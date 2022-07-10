use std::io::Write;

use unicode_xid::UnicodeXID;

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
_clap_complete_NAME() {
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
COMPREPLY=( $("COMPLETER" complete --index ${COMP_CWORD} --type ${COMP_TYPE} ${SPACE_ARG} --ifs="$IFS" -- "${COMP_WORDS[@]}") )
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
