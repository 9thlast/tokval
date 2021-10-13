tokval is a high-speed discord token validator.

# synopsis
```
tokval 2.2.1
by 9th
high-speed discord token validator
see https://github.com/9thlast/tokval for documentation

USAGE:
    tokval [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    enables verbose logging

OPTIONS:
    -i, --input <input_file>      file containing a line-separated list of tokens
    -j, --jobs <# jobs>           number of threads to spawn (defaults to # cpus available)
    -l, --log <log file>          file to output logs to
    -o, --output <output_file>    file to write all valid tokens to
    -p, --proxies <proxyfile>     file containing a line-separated list of proxies
```

# usage
the most commonly used command: `tokval --proxies proxies.txt -i tokens.txt -o valid.txt`

tokval is distributed as an executable with no dependencies, so just download it to some directory (ideally one in your PATH) and run it via the command-line. `tokval` can be run with the input/output sources as a file or the stdout/stderr streams.
- by default, running `tokval` will read tokens from stdin and write tokens to stdout; it will write logs to stderr
- tokval can be composed with other programs: `cat tokens.txt | tokval | program_that_uses_tokens`
- to read tokens from `input_file.txt` and output valid ones to `output_file.txt`: `tokval -i input_file.txt -o output_file.txt`
- a list of proxies may also be specified in a file and passed via the `--proxies` option: `tokval --proxies proxylist.txt`
- a logfile can be specified via `--log`: `tokval --log tokval.log`


# issues & questions
just submit any problems/questions to the issues page. pull requests/forks are welcome too.
