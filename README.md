# ratatar - random-access to tar archives

```
Usage:
  - ratatar
    Displays this help message.

  - ratatar <SUBCOMMAND> [OPTIONS]
    Invokes one of the subcommands outlined below.

Subcommands:
  - index
    Reads a tar file from standard input, generates an index for
    its content and writes it to standard output.

  - extract <TAR-FILE> <ENTRY-NAME> <OUT-FILE>
    Uses the index file to extract a file from a tar archive.

  - help
    Displays this help message.

Author:
    Alessandro Motta <alessandro.motta@brain.mpg.de>
```

## Basic usage

* Generating an index file during tarring  
`tar cf - /home/amotta | tee >(ratatar index > backup.tar.index) > backup.tar`
* Generating an index file for an existing tar archive  
`ratatar index < backup.tar > backup.tar.index`
* Extracting a file from a tar archive  
`ratatar extract backup.tar girl-with-hair-ribbon.png extracted.png`
* Displaying the content a file from a tar archive  
`ratatar extract backup.tar todo.txt -`

