# jot

## Install

Install with:

    cargo install --path . --force

Create `$HOME/.config/jot/config.toml` by running:

    jot

Edit `$HOME/.config/jot/config/toml` and give it a valid path to your journal.txt (you'll have to create it for now, absolute paths please).

## Notifications

When you run `jot notify` jot will attempt to notify you via your operating
systems notification tray (currently only Linux/Macos). We will notify you of any
notifications that haven't yet been made on this machine, and should have fired within
the last day. This prevents you from getting spammed with all notifications when you
setup a new machine, but notification will successfully fire on machines for which
the `journal.txt` file is synced.

To get this to happen automatically you can use a cron job to periodically fire
reminders (note you only get 1 minute resolution):

    crontab -e
    * * * * * /path/to/jot notify

## Usage

### Notes

Make a note:

    jot note

View all notes:

    jot notes

### Todos

Make a todo:

    jot todo

View all todos:

    jot todos

Complete a todo:

    jot complete 4


### Reminders

Make a reminder, note you need to setup notifications first:

    jot reminder in 10 minutes
    jot reminder sunday at noon
    jot reminder monday morning
    jot reminder tomorrow morning
    jot reminder Monday
    jot reminder Wed at 10
    jot reminder on sun at 10pm
    jot reminder sun 10:30pm
    jot reminder at 10:30pm
    jot reminder at noon
    jot reminder noon

View all reminders:

    jot reminders


### Tags

Any note/todo/reminder can have tags, a tag is just a word preceeded by
the `@` symbol.

List tags:

    jot tags

### Other

Dump everything:

    jot cat

Filter any of the view commands by tags:

    jot notes -t @music

Grep any of the view commands:

    jot todos -g birthday
    jot cat -g "foo.*?bar"

Edit a note/todo/reminder:

    jot edit 14

