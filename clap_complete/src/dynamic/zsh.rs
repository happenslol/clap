use std::io::Write;

use unicode_xid::UnicodeXID;

#[derive(Clone, Debug, clap::Args)]
#[allow(missing_docs)]
pub struct CompleteArgs {
}

/// Generate code to register the dynamic completion
pub fn register(
    name: &str,
    executables: impl IntoIterator<Item = impl AsRef<str>>,
    completer: &str,
    buf: &mut dyn Write,
) -> Result<(), std::io::Error> {
    let escaped_name = name.replace('-', "_");
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

    let completer = shlex::quote(completer);

    // Adapted from github.com/spf13/cobra
    // TODO: Implement no-space flag
    let script = r#"
#compdef _clap_complete_NAME NAME
# zsh completion for NAME
__clap_complete_NAME_debug() {
    local file="$BASH_COMP_DEBUG_FILE"
    if [[ -n ${file} ]]; then
        echo "$*" >> "${file}"
    fi
}

_clap_complete_NAME() {
    local lastParam lastChar flagPrefix compCmd compResult compLine lastComp
    local -a completions

    __clap_complete_NAME_debug "\n========= starting completion logic =========="
    __clap_complete_NAME_debug "CURRENT: ${CURRENT}, words[*]: ${words[*]}"

    # The user could have moved the cursor backwards on the command-line.
    # We need to trigger completion from the $CURRENT location, so we need
    # to truncate the command-line ($words) up to the $CURRENT location.
    # (We cannot use $CURSOR as its value does not work when a command is an alias.)
    words=("${=words[1,CURRENT]}")
    __clap_complete_NAME_debug "Truncated words[*]: ${words[*]},"
    lastParam=${words[-1]}
    lastChar=${lastParam[-1]}
    __clap_complete_NAME_debug "lastParam: ${lastParam}, lastChar: ${lastChar}"

    # For zsh, when completing a flag with an = (e.g., NAME -n=<TAB>)
    # completions must be prefixed with the flag
    setopt local_options BASH_REMATCH
    if [[ "${lastParam}" =~ '-.+=' ]]; then
        # We are dealing with a flag with an =
        flagPrefix="-P ${BASH_REMATCH}"
    fi

    # Prepare the command to obtain completions
    compCmd="COMPLETER complete --type 63 --index 1 --no-space -- ${words[1,-1]}"

    __clap_complete_NAME_debug "Calling completion command: eval ${compCmd}"

    # Use eval to handle any environment variables and such
    compResult=$(eval ${compCmd} 2>/dev/null)
    __clap_complete_NAME_debug "Completion command output: ${compResult}"

    __clap_complete_NAME_debug "completions: ${compResult}"
    __clap_complete_NAME_debug "flagPrefix: ${flagPrefix}"

    while IFS='\n' read -r compLine; do
        if [ -n "$compLine" ]; then
            # If requested, completions are returned with a description.
            # The description is preceded by a TAB character.
            # For zsh's _describe, we need to use a : instead of a TAB.
            # We first need to escape any : as part of the completion itself.
            compLine=${compLine//:/\\:}
            local tab="$(printf '\t')"
            compLine=${compLine//$tab/:}
            __clap_complete_NAME_debug "Adding completion: ${compLine}"
            completions+=${compLine}
            lastComp=$compLine
        fi
    done < <(printf "%%s\n" "${compResult[@]}")

    __clap_complete_NAME_debug "Calling _describe"
    if eval _describe "completions" completions; then
        __clap_complete_NAME_debug "_describe found some completions"
        # Return the success of having called _describe
        return 0
    else
        __clap_complete_NAME_debug "_describe did not find completions."
        __clap_complete_NAME_debug "Checking if we should do file completion."
        # TODO: Allow customizing behavior here

        # Perform file completion
        __clap_complete_NAME_debug "Activating file completion"

        # We must return the result of this command, so it must be the
        # last command, or else we must store its result to return it.
        _arguments '*:filename:_files'" ${flagPrefix}"
    fi
}

# don't run the completion function when being source-ed or eval-ed
if [ "$funcstack[1]" = "_NAME" ]; then
    _NAME
fi
"#
    .replace("NAME", &escaped_name)
    .replace("EXECUTABLES", &executables)
    .replace("COMPLETER", &completer)
    .replace("UPPER", &upper_name);

    writeln!(buf, "{}", script)?;
    Ok(())
}

/// The recommended file name for the registration code
pub fn file_name(name: &str) -> String {
    format!("{}.zsh", name)
}
