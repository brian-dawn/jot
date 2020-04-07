# jot

Hosted on [SourceHut](https://git.sr.ht/~brian-dawn/jot) but also hosted on [Github](https://github.com/brian-dawn/jot) for convenience. If you're curious why SourceHut is neat check out [this](https://sourcehut.org/blog/2019-10-23-srht-puts-users-first/).

## Install

If you don't have Rust installed an easy way is to use [rustup](https://rustup.rs/).

Build and install with:

    cargo install --path .

Initialize jot by running:

    jot

If you want to customize where the journal file lives
edit `$HOME/.config/jot/config.toml` and give it a valid path to your journal.txt (you'll have to create it for now, absolute paths please). 

I like putting my journal in a [Syncthing](https://syncthing.net/) (Dropbox would also work) folder so my config looks like this:

```
journal_path = "/Users/brian/Sync/journal.txt"
```

## MacOS Users

MacOS users already have a `jot` command installed. Make sure
`~/.cargo/bin` is in your `$PATH` or add the following to `.bashrc`:

    source $HOME/.cargo/env


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

Complete a todo (where `kw` is the id):

    jot complete kw


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

Edit a note/todo/reminder (where `bt` is the id):

    jot edit bt

Delete a note/todo/reminder (where 'fq' is the id):

    jot delete fq

## Configuration

Jot will use your default `$EDITOR` to determine how notes should be
written/edited. To update this, add the following to your `.bashrc` and update `vim` to be your desired editor:

    export EDITOR='vim'
    export VISUAL='vim'
