# easymux shell integration for bash.
#
# easymux starts bash as `bash --rcfile <this file>`, so bash loads this file
# instead of the user's ~/.bashrc. This file then:
#   1. loads the user's own startup files, the way bash normally would,
#   2. adds our hooks, after the user's config (so it can't undo them).
# --rcfile works on every bash version, including macOS's bash 3.2 (which
# ignores the `--posix` + ENV approach some other terminals use).
#
# The hooks tell easymux what the shell is doing, with invisible escape codes:
#   - right before a command runs: the command, as the title (OSC 2)
#   - right before each prompt:    the current folder (OSC 7)

# --- 1. The user's startup files, as bash would load them -------------------

if [[ -n "${EASYMUX_BASH_LOGIN-}" ]]; then
  # A login shell: /etc/profile, then the first of these that exists.
  builtin unset EASYMUX_BASH_LOGIN
  [[ -r /etc/profile ]] && builtin source /etc/profile
  for _easymux_file in ~/.bash_profile ~/.bash_login ~/.profile; do
    if [[ -r "$_easymux_file" ]]; then
      builtin source "$_easymux_file"
      break
    fi
  done
  builtin unset _easymux_file
else
  # A normal interactive shell: the user's ~/.bashrc. (On Debian/Ubuntu, bash
  # itself already loaded /etc/bash.bashrc before this file.)
  [[ -r ~/.bashrc ]] && builtin source ~/.bashrc
fi

# --- 2. Our hooks ------------------------------------------------------------

# Only once per shell.
[[ -n "${_easymux_loaded-}" ]] && builtin return 0
_easymux_loaded=1

# Turn a path into the URL form OSC 7 needs: "/Users/om/my docs" becomes
# "/Users/om/my%20docs". Letters, digits and / . _ ~ - stay as they are;
# every other byte becomes %XX.
_easymux_urlencode() {
  builtin local LC_ALL=C # go byte by byte, so non-English characters work too
  builtin local i char byte encoded=""
  for (( i = 0; i < ${#1}; i++ )); do
    char=${1:i:1}
    case $char in
      [a-zA-Z0-9/._~-]) encoded+=$char ;;
      *)
        builtin printf -v byte '%d' "'$char"
        # Old bash (3.2, macOS) reads bytes above 127 as negative numbers.
        (( byte < 0 )) && (( byte += 256 ))
        builtin printf -v char '%%%02X' "$byte"
        encoded+=$char
        ;;
    esac
  done
  builtin printf '%s' "$encoded"
}

# Before each prompt: report the current folder.
_easymux_prompt_command() {
  builtin local status=$? # keep the last command's exit code for the prompt
  builtin printf '\e]7;file://%s%s\a' "$HOSTNAME" "$(_easymux_urlencode "$PWD")"
  # The next command the user types is a new command line (see below).
  _easymux_waiting_for_command=1
  builtin return $status
}

# Before each command: use it as the title.
# bash has no "before a command" hook, so we use the DEBUG trap, which runs
# before EVERY simple command, including those inside PROMPT_COMMAND and
# every part of a pipeline. We only act on the first one after a prompt.
_easymux_preexec() {
  [[ -n "${_easymux_waiting_for_command-}" ]] || builtin return 0
  [[ -n "${COMP_LINE-}" ]] && builtin return 0 # tab completion, not a command
  [[ "$BASH_COMMAND" == _easymux_prompt_command* ]] && builtin return 0
  _easymux_waiting_for_command=
  builtin printf '\e]2;%s\a' "${BASH_COMMAND//[[:cntrl:]]/}"
}

# Add our prompt hook last. bash 5.1+ allows PROMPT_COMMAND to be a list.
if [[ "$(builtin declare -p PROMPT_COMMAND 2>/dev/null)" == "declare -a"* ]]; then
  PROMPT_COMMAND+=(_easymux_prompt_command)
else
  PROMPT_COMMAND="${PROMPT_COMMAND:+$PROMPT_COMMAND; }_easymux_prompt_command"
fi

# Add our command hook, unless the user already has a DEBUG trap (some tools
# install one): we never replace it. Folder titles still work without it.
if [[ -z "$(builtin trap -p DEBUG)" ]]; then
  builtin trap '_easymux_preexec' DEBUG
fi
