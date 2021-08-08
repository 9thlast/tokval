tokval is a high-speed discord token validator.

# synopsis
```
tokval 2.0.1
9th
high-speed discord token validator

USAGE:
    tokval [OPTIONS] <input_file> <output_file>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -j, --jobs <# jobs>          number of threads to spawn (defaults to the number of cpus available)
    -p, --proxies <proxyfile>    file containing a line-separated list of proxies

ARGS:
    <input_file>     file containing a line-separated list of tokens
    <output_file>    file to write all valid tokens to
```

# usage
tokval is distributed as an executable with no dependencies, so just download it to some directory and run it via the command-line: 
- \*nix systems: `./tokval --proxies proxyfile.txt input_file.txt output_file.txt`
- windows systems: `.\tokval.exe --proxies proxyfile.txt input_file.txt output_file.txt`

the `input_file.txt` here is a list of discord tokens, one per line. `output_file.txt` is the file to write all the valid tokens to (WARNING: tokval will overwrite all contents of `output_file.txt` so be careful!) additionally, tokval supports a `--proxy` option that allows you to specify a file containing a line-separated list of http proxies to use while validating tokens.

optionally, tokval may be installed to a location in your `PATH` and made available from the command-line anywhere

# issues & questions
just submit any problems/questions to the issues page. pull requests/forks are welcome too.
