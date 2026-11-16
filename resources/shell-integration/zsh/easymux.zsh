# easymux shell integration for zsh: the hooks.
#
# Tells easymux what the shell is doing, with invisible escape codes:
#   - right before a command runs: the command line, as the title (OSC 2)
#   - right before each prompt:    the current folder (OSC 7)
# easymux shows whichever came last as the tab title.

# Only once per shell.
(( ${+_easymux_loaded} )) && return
typeset -g _easymux_loaded=1

# Turn a path into the URL form OSC 7 needs: "/Users/om/my docs" becomes
# "/Users/om/my%20docs". Letters, digits and / . _ ~ - stay as they are;
# every other byte becomes %XX.
_easymux_urlencode() {
  emulate -L zsh
  local LC_ALL=C # go byte by byte, so non-English characters work too
  local i char encoded=""
  for (( i = 1; i <= ${#1}; i++ )); do
    char=${1[i]}
    case $char in
      [a-zA-Z0-9/._~-]) encoded+=$char ;;
      *) encoded+=$(printf '%%%02X' "'$char") ;;
    esac
  done
  print -rn -- "$encoded"
}

# Before each prompt: report the current folder.
_easymux_precmd() {
  # Stay the last prompt hook, so our report comes after any title the
  # user's config sets at the prompt (the most recent one wins).
  if [[ ${precmd_functions[-1]} != _easymux_precmd ]]; then
    precmd_functions=(${precmd_functions:#_easymux_precmd} _easymux_precmd)
  fi
  print -rn -- $'\e]7;file://'"${HOST}$(_easymux_urlencode "$PWD")"$'\a'
}

# Before each command: use the command line as the title.
# $1 is the command line; control characters are removed first.
_easymux_preexec() {
  print -rn -- $'\e]2;'"${1//[[:cntrl:]]}"$'\a'
}

typeset -ag precmd_functions preexec_functions
precmd_functions+=(_easymux_precmd)
preexec_functions+=(_easymux_preexec)
