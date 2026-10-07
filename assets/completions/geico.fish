# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_geico_global_optspecs
    string join \n h/help
end

function __fish_geico_needs_command
    # Figure out if the current invocation already has a command.
    set -l cmd (commandline -opc)
    set -e cmd[1]
    argparse -s (__fish_geico_global_optspecs) -- $cmd 2>/dev/null
    or return
    if set -q argv[1]
        # Also print the command, so this can be used to figure out what it is.
        echo $argv[1]
        return 1
    end
    return 0
end

function __fish_geico_using_subcommand
    set -l cmd (__fish_geico_needs_command)
    test -z "$cmd"
    and return 1
    contains -- $cmd[1] $argv
end

complete -c geico -n "__fish_geico_needs_command" -s h -l help -d 'Print help'
complete -c geico -n "__fish_geico_needs_command" -f -a "add" -d 'Register an executable and integrate it into the desktop and PATH'
complete -c geico -n "__fish_geico_needs_command" -f -a "list" -d 'List all managed applications with health checks'
complete -c geico -n "__fish_geico_needs_command" -f -a "show" -d 'Show detailed information and paths for a registered application'
complete -c geico -n "__fish_geico_needs_command" -f -a "update" -d 'Update metadata, target executable, icon, or categories of a registered application'
complete -c geico -n "__fish_geico_needs_command" -f -a "remove" -d 'Safely remove desktop launcher and symlink (preserves original executable)'
complete -c geico -n "__fish_geico_needs_command" -f -a "completions" -d 'Generate shell completion script (bash, zsh, fish, elvish, powershell)'
complete -c geico -n "__fish_geico_needs_command" -f -a "health" -d 'Check system environment and desktop integration health'
complete -c geico -n "__fish_geico_needs_command" -f -a "doctor" -d 'Audit and auto-repair broken symlinks, orphaned desktop files, and sandbox configuration'
complete -c geico -n "__fish_geico_needs_command" -f -a "run" -d 'Launch a registered application in the background'
complete -c geico -n "__fish_geico_needs_command" -f -a "watch" -d 'Scan and optionally monitor directories for AppImages'
complete -c geico -n "__fish_geico_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c geico -n "__fish_geico_using_subcommand add" -l name -d 'Display name of the application (defaults to executable filename)' -r
complete -c geico -n "__fish_geico_using_subcommand add" -l icon -d 'Path to the icon file (copied to ~/.local/share/icons)' -r -F
complete -c geico -n "__fish_geico_using_subcommand add" -l categories -d 'Desktop categories separated by commas (e.g. AudioVideo,Development). Omit the value to list the standard categories without registering anything' -r
complete -c geico -n "__fish_geico_using_subcommand add" -l bin-name -d 'Custom command name for the terminal symlink' -r
complete -c geico -n "__fish_geico_using_subcommand add" -l dest-bin -d 'Custom destination directory for the symlink (prompts sudo if outside $HOME)' -r -F
complete -c geico -n "__fish_geico_using_subcommand add" -l description -l comment -d 'Application description or tooltip (written as Comment in .desktop)' -r
complete -c geico -n "__fish_geico_using_subcommand add" -l no-desktop -d 'Skip creating a .desktop launcher entry'
complete -c geico -n "__fish_geico_using_subcommand add" -l no-symlink -d 'Skip creating a terminal $PATH symlink'
complete -c geico -n "__fish_geico_using_subcommand add" -l sandbox -l isolate -d 'Run the application in an isolated sandbox (bwrap or firejail)'
complete -c geico -n "__fish_geico_using_subcommand add" -l no-sandbox -d 'Disable sandboxing explicitly'
complete -c geico -n "__fish_geico_using_subcommand add" -s h -l help -d 'Print help'
complete -c geico -n "__fish_geico_using_subcommand list" -l category -r
complete -c geico -n "__fish_geico_using_subcommand list" -s i -l icons -l image -d 'Display application icon previews in compatible terminals'
complete -c geico -n "__fish_geico_using_subcommand list" -l no-icons -l no-image -d 'Suppress application icon previews even if supported by the terminal'
complete -c geico -n "__fish_geico_using_subcommand list" -s t -l table
complete -c geico -n "__fish_geico_using_subcommand list" -l broken
complete -c geico -n "__fish_geico_using_subcommand list" -l hide-disabled-symlinks -l hide-disabled -d 'Hide symlinks when they are disabled'
complete -c geico -n "__fish_geico_using_subcommand list" -l hide-target-ok -d 'Hide target status when it is ok'
complete -c geico -n "__fish_geico_using_subcommand list" -l hide-desktop-ok -d 'Hide desktop file status when it is ok'
complete -c geico -n "__fish_geico_using_subcommand list" -l hide-ok -l hide-ok-status -d 'Hide target and desktop statuses when they are ok'
complete -c geico -n "__fish_geico_using_subcommand list" -l description -l descriptions -l desc -d 'Display application descriptions in list output'
complete -c geico -n "__fish_geico_using_subcommand list" -l no-description -l no-descriptions -l no-desc -d 'Suppress application descriptions even if enabled in configuration'
complete -c geico -n "__fish_geico_using_subcommand list" -l ids -d 'Print only application IDs, one per line (useful for scripting)'
complete -c geico -n "__fish_geico_using_subcommand list" -l json -d 'Output the list of applications as a structured JSON array'
complete -c geico -n "__fish_geico_using_subcommand list" -s h -l help -d 'Print help'
complete -c geico -n "__fish_geico_using_subcommand show" -l bin -d 'Print only the target executable path'
complete -c geico -n "__fish_geico_using_subcommand show" -l desktop -d 'Print only the desktop file path'
complete -c geico -n "__fish_geico_using_subcommand show" -l icon -d 'Print only the icon path'
complete -c geico -n "__fish_geico_using_subcommand show" -l symlink -d 'Print only the terminal symlink path'
complete -c geico -n "__fish_geico_using_subcommand show" -l json -d 'Print the application entry as JSON'
complete -c geico -n "__fish_geico_using_subcommand show" -s a -l absolute -l full-path -d 'Print full absolute paths instead of collapsing home directory to ~/'
complete -c geico -n "__fish_geico_using_subcommand show" -s i -l icons -l image -d 'Display application icon preview in the terminal (default in Kitty/Ghostty)'
complete -c geico -n "__fish_geico_using_subcommand show" -l no-icons -l no-image -d 'Suppress terminal image preview even if supported by the terminal'
complete -c geico -n "__fish_geico_using_subcommand show" -s h -l help -d 'Print help'
complete -c geico -n "__fish_geico_using_subcommand update" -l target -d 'New path to the executable file' -r -F
complete -c geico -n "__fish_geico_using_subcommand update" -l icon -d 'New path to the icon file' -r -F
complete -c geico -n "__fish_geico_using_subcommand update" -l name -d 'New display name' -r
complete -c geico -n "__fish_geico_using_subcommand update" -l categories -d 'New desktop categories separated by commas (e.g. AudioVideo,Development). Omit the value to show the current categories without changing anything' -r
complete -c geico -n "__fish_geico_using_subcommand update" -l description -l comment -d 'New application description or tooltip' -r
complete -c geico -n "__fish_geico_using_subcommand update" -l remove-symlink -d 'Remove the terminal symlink for this application'
complete -c geico -n "__fish_geico_using_subcommand update" -l sandbox -l isolate -d 'Run the application in an isolated sandbox (bwrap or firejail)'
complete -c geico -n "__fish_geico_using_subcommand update" -l no-sandbox -d 'Disable sandboxing explicitly'
complete -c geico -n "__fish_geico_using_subcommand update" -s h -l help -d 'Print help'
complete -c geico -n "__fish_geico_using_subcommand remove" -l all -d 'Remove all registered applications from the system'
complete -c geico -n "__fish_geico_using_subcommand remove" -l delete-target -l delete-appimage -d 'Also delete the original target executable (AppImage or binary) from disk'
complete -c geico -n "__fish_geico_using_subcommand remove" -s y -l yes -d 'Skip the confirmation prompt when deleting the target executable'
complete -c geico -n "__fish_geico_using_subcommand remove" -s h -l help -d 'Print help'
complete -c geico -n "__fish_geico_using_subcommand completions" -l install -d 'Install completion script automatically into user shell directory (default behavior)'
complete -c geico -n "__fish_geico_using_subcommand completions" -l stdout -d 'Print raw completion script to stdout instead of installing to disk'
complete -c geico -n "__fish_geico_using_subcommand completions" -s h -l help -d 'Print help'
complete -c geico -n "__fish_geico_using_subcommand health" -l json
complete -c geico -n "__fish_geico_using_subcommand health" -s v -l verbose -d 'List every checked tool individually (missing ones as dim `[–]`)'
complete -c geico -n "__fish_geico_using_subcommand health" -s h -l help -d 'Print help'
complete -c geico -n "__fish_geico_using_subcommand doctor" -l fix -d 'Automatically repair detected issues (clean up dead entries, prune broken symlinks)'
complete -c geico -n "__fish_geico_using_subcommand doctor" -s y -l yes -d 'Confirm without interactive prompt'
complete -c geico -n "__fish_geico_using_subcommand doctor" -s h -l help -d 'Print help'
complete -c geico -n "__fish_geico_using_subcommand run" -s h -l help -d 'Print help'
complete -c geico -n "__fish_geico_using_subcommand watch" -l icons-dir -d 'Directory containing icons matched by AppImage name' -r -F
complete -c geico -n "__fish_geico_using_subcommand watch" -l continuous -l foreground -d 'Keep running filesystem watcher in foreground (alias: --foreground)'
complete -c geico -n "__fish_geico_using_subcommand watch" -l scan-only -d 'Scan once and exit (alias: --once)'
complete -c geico -n "__fish_geico_using_subcommand watch" -l recursive -d 'Monitor subdirectories recursively'
complete -c geico -n "__fish_geico_using_subcommand watch" -l dry-run -d 'Show registrations without changing the registry or filesystem'
complete -c geico -n "__fish_geico_using_subcommand watch" -s h -l help -d 'Print help'
complete -c geico -n "__fish_geico_using_subcommand help; and not __fish_seen_subcommand_from add list show update remove completions health doctor run watch help" -f -a "add" -d 'Register an executable and integrate it into the desktop and PATH'
complete -c geico -n "__fish_geico_using_subcommand help; and not __fish_seen_subcommand_from add list show update remove completions health doctor run watch help" -f -a "list" -d 'List all managed applications with health checks'
complete -c geico -n "__fish_geico_using_subcommand help; and not __fish_seen_subcommand_from add list show update remove completions health doctor run watch help" -f -a "show" -d 'Show detailed information and paths for a registered application'
complete -c geico -n "__fish_geico_using_subcommand help; and not __fish_seen_subcommand_from add list show update remove completions health doctor run watch help" -f -a "update" -d 'Update metadata, target executable, icon, or categories of a registered application'
complete -c geico -n "__fish_geico_using_subcommand help; and not __fish_seen_subcommand_from add list show update remove completions health doctor run watch help" -f -a "remove" -d 'Safely remove desktop launcher and symlink (preserves original executable)'
complete -c geico -n "__fish_geico_using_subcommand help; and not __fish_seen_subcommand_from add list show update remove completions health doctor run watch help" -f -a "completions" -d 'Generate shell completion script (bash, zsh, fish, elvish, powershell)'
complete -c geico -n "__fish_geico_using_subcommand help; and not __fish_seen_subcommand_from add list show update remove completions health doctor run watch help" -f -a "health" -d 'Check system environment and desktop integration health'
complete -c geico -n "__fish_geico_using_subcommand help; and not __fish_seen_subcommand_from add list show update remove completions health doctor run watch help" -f -a "doctor" -d 'Audit and auto-repair broken symlinks, orphaned desktop files, and sandbox configuration'
complete -c geico -n "__fish_geico_using_subcommand help; and not __fish_seen_subcommand_from add list show update remove completions health doctor run watch help" -f -a "run" -d 'Launch a registered application in the background'
complete -c geico -n "__fish_geico_using_subcommand help; and not __fish_seen_subcommand_from add list show update remove completions health doctor run watch help" -f -a "watch" -d 'Scan and optionally monitor directories for AppImages'
complete -c geico -n "__fish_geico_using_subcommand help; and not __fish_seen_subcommand_from add list show update remove completions health doctor run watch help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
