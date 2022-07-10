use std::io::Write;

use unicode_xid::UnicodeXID;

/// Generate code to register the dynamic completion
pub fn register(
    name: &str,
    executables: impl IntoIterator<Item = impl AsRef<str>>,
    completer: &str,
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

    let completer = shlex::quote(completer);

    // Adapted from github.com/spf13/cobra
    // TODO: Implement no-space flag
    let script = r#"
#compdef NAME
# zsh completion for NAME -*- shell-script -*-
__NAME_debug() {
    local file="$BASH_COMP_DEBUG_FILE"
    if [[ -n ${file} ]]; then
        echo "$*" >> "${file}"
    fi
}

_NAME() {
    local lastParam lastChar flagPrefix compCmd compResult compLine lastComp
    local -a completions

    __NAME_debug "\n========= starting completion logic =========="
    __NAME_debug "CURRENT: ${CURRENT}, words[*]: ${words[*]}"

    # The user could have moved the cursor backwards on the command-line.
    # We need to trigger completion from the $CURRENT location, so we need
    # to truncate the command-line ($words) up to the $CURRENT location.
    # (We cannot use $CURSOR as its value does not work when a command is an alias.)
    words=("${=words[1,CURRENT]}")
    __NAME_debug "Truncated words[*]: ${words[*]},"
    lastParam=${words[-1]}
    lastChar=${lastParam[-1]}
    __NAME_debug "lastParam: ${lastParam}, lastChar: ${lastChar}"

    # For zsh, when completing a flag with an = (e.g., NAME -n=<TAB>)
    # completions must be prefixed with the flag
    setopt local_options BASH_REMATCH
    if [[ "${lastParam}" =~ '-.+=' ]]; then
        # We are dealing with a flag with an =
        flagPrefix="-P ${BASH_REMATCH}"
    fi

    # Prepare the command to obtain completions
    compCmd="${words[1]} COMPLETER ${words[2,-1]}"
    if [ "${lastChar}" = "" ]; then
        # If the last parameter is complete (there is a space following it)
        # We add an extra empty parameter so we can indicate this to the go completion code.
        __NAME_debug "Adding extra empty parameter"
        compCmd="${compCmd} \"\""
    fi

    __NAME_debug "About to call: eval ${compCmd}"

    # Use eval to handle any environment variables and such
    compResult=$(eval ${compCmd} 2>/dev/null)
    __NAME_debug "completion output: ${compResult}"

    __NAME_debug "completions: ${compResult}"
    __NAME_debug "flagPrefix: ${flagPrefix}"

    while IFS='\n' read -r compLine; do
        if [ -n "$compLine" ]; then
            # If requested, completions are returned with a description.
            # The description is preceded by a TAB character.
            # For zsh's _describe, we need to use a : instead of a TAB.
            # We first need to escape any : as part of the completion itself.
            compLine=${compLine//:/\\:}
            local tab="$(printf '\t')"
            compLine=${compLine//$tab/:}
            __NAME_debug "Adding completion: ${compLine}"
            completions+=${compLine}
            lastComp=$compLine
        fi
    done < <(printf "%%s\n" "${compResult[@]}")

    __NAME_debug "Calling _describe"
    if eval _describe "completions" completions; then
        __NAME_debug "_describe found some completions"
        # Return the success of having called _describe
        return 0
    else
        __NAME_debug "_describe did not find completions."
        __NAME_debug "Checking if we should do file completion."
        # TODO: Allow customizing behavior here

        # Perform file completion
        __NAME_debug "Activating file completion"

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
