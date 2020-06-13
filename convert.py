import re

import sys


journal_path = sys.argv[1]
dir_path = sys.argv[2]

counter = 0

def write_jot(contents):

    global counter
    counter = counter + 1

    with open("{}/{:014}.jot".format(dir_path, counter), 'w') as out:
        out.write(contents)

with open(journal_path) as journal:

    all_lines = journal.read().splitlines()

    wip_jot = ''

    reg = '\[(\d\d\d\d\-\d\d\-\d\dT\d\d:\d\d:\d\d-\d\d:\d\d)(.*?)\].*'
    for line in all_lines:
        found = re.search(reg, line)
        if found:
            # Write out wip_jot
            print(wip_jot)
            print("#############")
            write_jot(wip_jot)
            wip_jot = line
        else:
            wip_jot = wip_jot + '\n'
            wip_jot = wip_jot + line

    print(wip_jot)
    write_jot(wip_jot)
    print('************')

# write out wip_jot

