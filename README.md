tokval is a high-speed discord token validator.


# usage
tokval is distributed as an executable with no dependencies, so just download it to some directory and run it via the command-line: 
- \*nix systems: `./tokval input_file.txt output_file.txt`
- windows systems: `.\tokval.exe input_file.txt output_file.txt`

the `input_file.txt` here is a list of discord tokens, one per line. `output_file.txt` is the file to write all the valid tokens to (WARNING: tokval will overwrite all contents of `output_file.txt` so be careful!)

optionally, tokval may be installed to a location in your `PATH` and made available from the command-line anywhere

# issues & questions
just submit any problems/questions to the issues page. pull requests/forks are welcome too.
