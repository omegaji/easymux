# easymux shell integration for zsh: the startup file zsh loads first.
#
# easymux starts zsh with ZDOTDIR pointing at this folder, so zsh loads this
# file instead of the user's .zshenv. We immediately put ZDOTDIR back to the
# user's real value and load their own .zshenv, so their config runs exactly
# as usual (zsh then loads their .zshrc etc. from the restored ZDOTDIR).
# Finally we add our own hooks (easymux.zsh).

if [[ -n "${EASYMUX_ZSH_ZDOTDIR+X}" ]]; then
  # The user had their own ZDOTDIR: restore it.
  builtin export ZDOTDIR="$EASYMUX_ZSH_ZDOTDIR"
  builtin unset EASYMUX_ZSH_ZDOTDIR
else
  # The user had none: zsh then uses the home folder, as normal.
  builtin unset ZDOTDIR
fi

# Load the user's own .zshenv, exactly as zsh would have.
if [[ -r "${ZDOTDIR:-$HOME}/.zshenv" ]]; then
  builtin source "${ZDOTDIR:-$HOME}/.zshenv"
fi

# Add our hooks (only for interactive shells: the ones showing a prompt).
if [[ -o interactive && -r "${EASYMUX_SHELL_INTEGRATION_DIR}/zsh/easymux.zsh" ]]; then
  builtin source "${EASYMUX_SHELL_INTEGRATION_DIR}/zsh/easymux.zsh"
fi
