# CWT Log Analyser

A command-line tool for analysing ADIF log files from CWT (CWops Weekly Test) amateur radio contests.

This is just a fun thing I made to paste some info into 3830 when I post my score.  If you have an idea for something to add to it, let me know!

## Features

- **QSO Statistics**: Reports total number of QSOs in the log
- **Duplicate Detection**: Identifies duplicate contacts (same callsign on same band)
- **Rate Analysis**: Finds the best 10-minute operating window by contact count
- **CWops Membership Check**: Optionally validates contacts against the live CWops member roster, identifying which stations worked are CWops members and listing non-members

The CWops membership check handles callsign variants with prefixes and suffixes (e.g., `N9UNX/3` or `VE3/N9UNX` will correctly match the member callsign `N9UNX`).

## Usage

```
cwt_log_analyser [OPTIONS] <adif_file>

Options:
  -c, --cwops    Check contacts against CWops member roster
  -v, --verbose  Enable verbose output
```

### Examples

Basic log analysis:
```
cwt_log_analyser mylog.adi
```

With CWops membership check:
```
cwt_log_analyser --cwops mylog.adi
```

## Releases

Pre-built binaries (windows, mac, linux) are available on the [Releases page](https://github.com/chadsbrown/cwt_log_analyser/releases).

## Building

```
cargo build --release
```

The binary will be available at `target/release/cwt_log_analyser`.

## Acknowledgements

This application was created almost entirely with [Claude Opus](https://www.anthropic.com/claude), Anthropic's AI assistant. The implementation, including ADIF parsing, CWops roster fetching, statistical analysis, and the GitHub Actions release workflow, was generated through collaborative prompting with Claude.
