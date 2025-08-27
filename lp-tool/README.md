# Line Protocol CLI Tool

A command-line tool that reads InfluxDB line protocol files, adds custom tags, and writes the processed data to a new line protocol file.


## Installation

1. Clone or download the source code
2. Initialize the Go module and download dependencies:

    ```bash
    go mod tidy
    ```

3. Build the executable:

    ```bash
    go build -C lp-tool -o lp-tool
    ```

## Usage

```bash
./lp-tool [options]
```

### Options

- `-input`: Input line protocol file path (required)
- `-output`: Output line protocol file path (default "\<input file\>.tmp.influx") 
- `-tag`: Add a tag in format 'key=value' (can be used multiple times)


### Examples

Basic usage with a default output file:
```bash
./lp-tool -input metrics.lp -tag run_id=123456789
```

Advanced usage with multiple custom tags:
```bash
./lp-tool -input metrics.lp -output tagged_metrics.lp -tag env=prod -tag region=europe -tag run_id=123456789
```

## Dependencies

- `github.com/influxdata/telegraf`: For line protocol parsing, metric manipulation, and serialization.
