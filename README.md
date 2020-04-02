# jot

## Install

Install with:

    cargo install --path . --force

Create `$HOME/.config/jot/config.toml` by running:

    jot

Edit `$HOME/.config/jot/config/toml` and give it a valid path to your journal.txt (you'll have to create it for now, absolute paths please).


## Usage

Make a note:

    jot note

Make a todo:

    jot todo

Make a reminder, note you need to setup notifications first:

    jot reminder in 10 minutes

List tags (TODO: should be jot tags):

    jot tag

Dump everything:

    jot cat

Grep for a regex pattern:

    jot grep hello
    jot grep @my-tag

## Notifications

    crontab -e
    * * * * * /path/to/jot notify
