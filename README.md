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

Make a note:

    jot note

Make a todo:

    jot todo

Make a reminder, note you need to setup notifications first:

    jot reminder in 10 minutes

List tags:

    jot tags

Dump everything:

    jot cat

View all notes:

    jot notes

View all todos:

    jot todos

View all reminders:

    jot reminders

Filter any of the view commands by tags:

    jot notes -t @music

Grep any of the view commands:

    jot todos -g birthday
    jot cat -g "foo.*?bar"


