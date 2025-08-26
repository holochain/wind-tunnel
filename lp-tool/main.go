package main

import (
	"bufio"
	"flag"
	"fmt"
	"os"
	"strings"

	"github.com/influxdata/telegraf/plugins/parsers/influx"
	influxSer "github.com/influxdata/telegraf/plugins/serializers/influx"
)


// Custom type to handle multiple tag flags
type TagMap map[string]string

// Implement String for TagMap ; required by flag.Value interface
func (tm TagMap) String() string {
	var pairs []string
	for k, v := range tm {
		pairs = append(pairs, fmt.Sprintf("%s=%s", k, v))
	}
	return strings.Join(pairs, ", ")
}

// Implement Set for TagMap ; required by flag.Value interface
func (tm TagMap) Set(value string) error {
	parts := strings.SplitN(value, "=", 2)
	if len(parts) != 2 {
		return fmt.Errorf("tag must be in format 'key=value', got: %s", value)
	}

	key := strings.TrimSpace(parts[0])
	val := strings.TrimSpace(parts[1])

	if key == "" {
		return fmt.Errorf("tag key cannot be empty")
	}

	if val == "" {
		return fmt.Errorf("tag value cannot be empty")
	}

    if _, exists := tm[key]; exists {
		return fmt.Errorf("tag key already provided")
	}

	tm[key] = val

	return nil
}

type Config struct {
	InputFile  string
	OutputFile string
    Tags TagMap
}

func main() {
    config, err := parseFlags()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		flag.Usage()
		os.Exit(1)
	}
	if err := processLineProtocol(config); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("Successfully processed %s and wrote to %s\n", config.InputFile, config.OutputFile)
}

func parseFlags() (Config, error) {
	var config Config
	config.Tags = make(TagMap)

	flag.StringVar(&config.InputFile, "input", "holochain.influx", "Input line protocol file path")
	flag.StringVar(&config.OutputFile, "output", "holochain.tmp.influx", "Output line protocol file path")
	flag.Var(&config.Tags, "tag", "Add a tag in format 'key=value' (can be used multiple times)")

	flag.Usage = func() {
		fmt.Fprintf(os.Stderr, "Usage: %s [options]\n\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "A CLI tool to process InfluxDB line protocol files for adding custom tags.\n\n")
		fmt.Fprintf(os.Stderr, "Options:\n")
		flag.PrintDefaults()
		fmt.Fprintf(os.Stderr, "\nExample:\n")
		fmt.Fprintf(os.Stderr, "  %s -input data.influx -output holochain.influx -tag env=production -tag region=europe -tag version=1.2.3\n", os.Args[0])
	}

	flag.Parse()

	if config.InputFile == "" || len(config.Tags) == 0 {
		return config, fmt.Errorf("input file or tags not specified")
	}
    if config.OutputFile == "" {
        config.OutputFile = config.InputFile + ".tmp.influx"
    }
    if config.OutputFile == config.InputFile {
		return config, fmt.Errorf("input and output files must be different.")
	}
	return config, nil
}

func processLineProtocol(config Config) error {
	// Open input file
	inputFile, err := os.Open(config.InputFile)
	if err != nil {
		return fmt.Errorf("failed to open input file: %w", err)
	}
	defer inputFile.Close()

	// Create output file
	outputFile, err := os.Create(config.OutputFile)
	if err != nil {
		return fmt.Errorf("failed to create output file: %w", err)
	}
	defer outputFile.Close()

	// Initialize the InfluxDB line protocol parser
	parser := &influx.Parser{}
	if err := parser.Init(); err != nil {
		return fmt.Errorf("failed to initialize parser: %w", err)
	}

	// Initialize the InfluxDB line protocol serializer
	serializer := &influxSer.Serializer{}
	if err := serializer.Init(); err != nil {
		return fmt.Errorf("failed to initialize serializer: %w", err)
	}

	scanner := bufio.NewScanner(inputFile)
	writer := bufio.NewWriter(outputFile)
	defer writer.Flush()

	lineCount := 0
	processedCount := 0

	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		lineCount++

		// Skip empty lines and comments
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}

		// Parse the line protocol using the parser plugin
		metrics, err := parser.Parse([]byte(line))
		if err != nil {
			fmt.Fprintf(os.Stderr, "Warning: Failed to parse line %d: %v\n", lineCount, err)
			continue
		}

		for _, telegrafMetric := range metrics {
			// Add custom tags
            for key, value := range config.Tags {
                // Warn if tag already exists
                if tag, has := telegrafMetric.GetTag(key) ; has {
                	fmt.Fprintf(os.Stderr, "[%d] Warning: tag %s=%s has been overwritten\n", lineCount, key, tag)
                }
                // Add (or replace) tag
			    telegrafMetric.AddTag(key, value)
            }

			// Convert back to line protocol using serializer
			serializedBytes, err := serializer.Serialize(telegrafMetric)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Warning: Failed to serialize metric on line %d: %v\n", lineCount, err)
				continue
			}

			// Write to output file
			if _, err := writer.Write(serializedBytes); err != nil {
				return fmt.Errorf("failed to write to output file: %w", err)
			}

			processedCount++
		}
	}

	if err := scanner.Err(); err != nil {
		return fmt.Errorf("error reading input file: %w", err)
	}

	fmt.Printf("Processed %d metrics from %d lines\n", processedCount, lineCount)
	return nil
}
