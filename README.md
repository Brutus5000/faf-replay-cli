# faf-replay-cli
A tiny command line tool written in Rust to launch FAF replays in parallel to the FAF client.

## Usage
You need to have the right game files loaded already by the client.

Here is the auto-generated help:
```
USAGE:
    faf-replay-cli [OPTIONS] --executable <PATH TO ForgedAlliance.exe> --local-file <FILE>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -e, --executable <PATH TO ForgedAlliance.exe>    Path to the ForgedAlliance.exe
    -f, --local-file <FILE>                          Path to the replay file you want to watch
    -w, --wrapper <WRAPPER>                          Path to the wrapper script (usually for Linux)

```
