package main

import (
	"flag"
	"io"
	"os"
	"path/filepath"
	"strings"
	"reflect"
	"testing"
)


func TestTagMapSet(t *testing.T) {
	tests := []struct {
		name        string
		input       string
		expectedErr bool
		expectedMap TagMap
	}{
		{
			name:        "valid tag",
			input:       "key=value",
			expectedErr: false,
			expectedMap: TagMap{"key": "value"},
		},
		{
			name:        "invalid format",
			input:       "keyvalue",
			expectedErr: true,
			expectedMap: TagMap{},
		},
        {
            name:        "invalid format 2",
            input:       "key value",
            expectedErr: true,
            expectedMap: TagMap{},
        },
		{
			name:        "empty key",
			input:       "=value",
			expectedErr: true,
			expectedMap: TagMap{},
		},
		{
			name:        "empty value",
			input:       "key=",
			expectedErr: true,
			expectedMap: TagMap{},
		},
		{
			name:        "spaces around key and value",
			input:       " key = value ",
			expectedErr: false,
			expectedMap: TagMap{"key": "value"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			tm := make(TagMap)
			err := tm.Set(tt.input)

			if tt.expectedErr && err == nil {
				t.Errorf("TagMap.Set() should have returned an error for input: %s", tt.input)
			} else if !tt.expectedErr && err != nil {
				t.Errorf("TagMap.Set() returned unexpected error: %v", err)
			}

			if !tt.expectedErr {
				for k, v := range tt.expectedMap {
					actualValue, exists := tm[k]
					if !exists {
						t.Errorf("Expected key %s not found in map", k)
					} else if actualValue != v {
						t.Errorf("Value mismatch for key %s: got %s, expected %s", k, actualValue, v)
					}
				}
			}
		})
	}
}

func TestParseFlags(t *testing.T) {
	// Save original flag set and command line arguments to restore them later
	originalFlagCommandLine := flag.CommandLine
	originalArgs := os.Args

	// Helper function to reset flags and args between test cases
	resetFlags := func() {
		flag.CommandLine = flag.NewFlagSet(os.Args[0], flag.ExitOnError)
		os.Args = originalArgs
	}

	// Defer restoring original flag set and args
	defer func() {
		flag.CommandLine = originalFlagCommandLine
		os.Args = originalArgs
	}()

	tests := []struct {
		name       string
		args       []string
		wantConfig Config
		wantErr    bool
	}{
		{
			name: "default values with tags",
			args: []string{"cmd", "-tag", "env=prod"},
			wantConfig: Config{
				InputFile:  "holochain.influx",
				OutputFile: "holochain.tmp.influx",
				Tags:       TagMap{"env": "prod"},
			},
			wantErr: true,
		},
		{
			name: "empty output file should use input as base and remove previous ext",
			args: []string{"cmd", "-input", "holochain.influx", "-tag", "env=prod"},
			wantConfig: Config{
				InputFile:  "holochain.influx",
				OutputFile: "holochain.tmp.influx",
				Tags:       TagMap{"env": "prod"},
			},
			wantErr: false,
		},
        {
            name: "empty output file should use input as base",
            args: []string{"cmd", "-input", "holochain", "-tag", "env=prod"},
            wantConfig: Config{
                InputFile:  "holochain",
                OutputFile: "holochain.tmp.influx",
                Tags:       TagMap{"env": "prod"},
            },
            wantErr: false,
        },
		{
			name: "custom input and output with multiple tags",
			args: []string{"cmd", "-input", "custom.influx", "-output", "result.influx", "-tag", "env=dev", "-tag", "region=eu"},
			wantConfig: Config{
				InputFile:  "custom.influx",
				OutputFile: "result.influx",
				Tags:       TagMap{"env": "dev", "region": "eu"},
			},
			wantErr: false,
		},
		{
			name:       "missing tags",
			args:       []string{"cmd", "-input", "data.influx"},
			wantConfig: Config{InputFile: "data.influx", Tags: TagMap{}},
			wantErr:    true,
		},
		{
			name:       "empty input file",
			args:       []string{"cmd", "-input", "", "-tag", "env=test"},
			wantConfig: Config{InputFile: "", Tags: TagMap{"env": "test"}},
			wantErr:    true,
		},
		{
			name: "empty output file",
			args: []string{"cmd", "-input", "data.influx", "-output", "", "-tag", "env=test"},
			wantConfig: Config{
				InputFile:  "data.influx",
				OutputFile: "data.influx.tmp.influx",
				Tags:       TagMap{"env": "test"},
			},
			wantErr: true,
		},
		{
			name:       "same input and output files",
			args:       []string{"cmd", "-input", "same.influx", "-output", "same.influx", "-tag", "env=test"},
			wantConfig: Config{InputFile: "same.influx", OutputFile: "same.influx", Tags: TagMap{"env": "test"}},
			wantErr:    true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Reset flags and set args for this test case
			resetFlags()
			os.Args = tt.args

			gotConfig, err := parseFlags()

			// Check if error status matches expectation
			if (err != nil) != tt.wantErr {
				t.Errorf("parseFlags() error = %v, wantErr %v", err, tt.wantErr)
				return
			}

			// If no error is expected, check if config matches expectation
			if !tt.wantErr && !reflect.DeepEqual(gotConfig, tt.wantConfig) {
				t.Errorf("parseFlags() = %+v, want %+v", gotConfig, tt.wantConfig)
			}
		})
	}
}



func TestProcessLineProtocol(t *testing.T) {
	// Create temporary directory for test files
	tempDir, err := os.MkdirTemp("", "test")
	if err != nil {
		t.Fatalf("Failed to create temp directory: %v", err)
	}
	defer os.RemoveAll(tempDir)

	// Create test input file
	inputData := `
# Comment line
measurement,tag1=value1 field1=42 1600000000000000000
measurement field1=43 1600000000000000000
empty_line

measurement,tag2=value2 field1=44 1600000000000000000
invalid line
`
	inputFile := filepath.Join(tempDir, "input.influx")
	if err := os.WriteFile(inputFile, []byte(inputData), 0644); err != nil {
		t.Fatalf("Failed to create test input file: %v", err)
	}

	outputFile := filepath.Join(tempDir, "output.influx")

	// Redirect stderr for capturing warnings
	oldStderr := os.Stderr
	oldStdout := os.Stdout
	defer func() {
		os.Stderr = oldStderr
		os.Stdout = oldStdout
	}()

	stderrR, stderrW, _ := os.Pipe()
	stdoutR, stdoutW, _ := os.Pipe()
	os.Stderr = stderrW
	os.Stdout = stdoutW

	// Process the test file
	config := Config{
		InputFile:  inputFile,
		OutputFile: outputFile,
		Tags:       TagMap{"env": "test", "region": "local"},
	}

	err = processLineProtocol(config)

	// Close writers to capture output
	stderrW.Close()
	stdoutW.Close()

	// Read captured output
	stderrOutput, _ := io.ReadAll(stderrR)
	stdoutOutput, _ := io.ReadAll(stdoutR)

	// Restore stderr and stdout
	os.Stderr = oldStderr
	os.Stdout = oldStdout

	// Check for errors
	if err != nil {
		t.Fatalf("processLineProtocol returned error: %v", err)
	}

	// Check if output file exists
	if _, err := os.Stat(outputFile); os.IsNotExist(err) {
		t.Fatalf("Output file was not created")
	}

	// Read output file content
	outputContent, err := os.ReadFile(outputFile)
	if err != nil {
		t.Fatalf("Failed to read output file: %v", err)
	}

	// Check if output contains our custom tags
	outputStr := string(outputContent)
	if !strings.Contains(outputStr, "env=test") || !strings.Contains(outputStr, "region=local") {
		t.Errorf("Output doesn't contain expected tags. Output: %s", outputStr)
	}

	// Verify stdout contains processing stats
	if !strings.Contains(string(stdoutOutput), "Processed") {
		t.Errorf("Expected processing stats in stdout, got: %s", string(stdoutOutput))
	}

	// Check stderr for warnings about the invalid line
	if !strings.Contains(string(stderrOutput), "Failed to parse line") {
		t.Errorf("Expected warning about invalid line, got: %s", string(stderrOutput))
	}
}


// TestProcessLineProtocolWithExistingTags tests the tag overwrite warning
func TestProcessLineProtocolWithExistingTags(t *testing.T) {
	// Create temporary directory for test files
	tempDir, err := os.MkdirTemp("", "test-tags")
	if err != nil {
		t.Fatalf("Failed to create temp directory: %v", err)
	}
	defer os.RemoveAll(tempDir)

	// Create test input file with tags that will be overwritten
	inputData := `measurement,env=prod field1=42 1600000000000000000`
	inputFile := filepath.Join(tempDir, "input-tags.influx")
	if err := os.WriteFile(inputFile, []byte(inputData), 0644); err != nil {
		t.Fatalf("Failed to create test input file: %v", err)
	}

	outputFile := filepath.Join(tempDir, "output-tags.influx")

	// Capture stderr to check for warnings
	oldStderr := os.Stderr
	r, w, _ := os.Pipe()
	os.Stderr = w
	defer func() { os.Stderr = oldStderr }()

	// Process the test file with tags that will overwrite existing ones
	config := Config{
		InputFile:  inputFile,
		OutputFile: outputFile,
		Tags:       TagMap{"env": "test"},
	}

	err = processLineProtocol(config)

	// Close the writer to capture output
	w.Close()
	stderr, _ := io.ReadAll(r)

	// Check for errors
	if err != nil {
		t.Fatalf("processLineProtocol returned error: %v", err)
	}

	// Verify there's a warning about overwriting tags
	if !strings.Contains(string(stderr), "tag env=prod has been overwritten") {
		t.Errorf("Expected warning about overwritten tag, got: %s", string(stderr))
	}

	// Read output file content
	outputContent, err := os.ReadFile(outputFile)
	if err != nil {
		t.Fatalf("Failed to read output file: %v", err)
	}

	// Verify the tag was actually overwritten
	if !strings.Contains(string(outputContent), "env=test") {
		t.Errorf("Tag was not properly overwritten in output")
	}
	if strings.Contains(string(outputContent), "env=prod") {
		t.Errorf("Original tag should be overwritten but is still present")
	}
}

// TestProcessLineProtocolFileErrors tests error handling for file operations
func TestProcessLineProtocolFileErrors(t *testing.T) {
	// Test input file doesn't exist
	config := Config{
		InputFile:  "nonexistent-file.influx",
		OutputFile: "output.influx",
		Tags:       TagMap{"key": "value"},
	}
	err := processLineProtocol(config)
	if err == nil {
		t.Errorf("Expected error for nonexistent input file, got nil")
	}
	if err != nil && !strings.Contains(err.Error(), "failed to open input file") {
		t.Errorf("Expected 'failed to open input file' error, got: %v", err)
	}

	// Create a directory that can't be written to
	readOnlyDir, err := os.MkdirTemp("", "readonly")
	if err != nil {
		t.Fatalf("Failed to create temp directory: %v", err)
	}
	defer os.RemoveAll(readOnlyDir)

	// Create a valid input file
	inputFile := filepath.Join(readOnlyDir, "input.influx")
	if err := os.WriteFile(inputFile, []byte("measurement field=1 1600000000000000000"), 0644); err != nil {
		t.Fatalf("Failed to create test input file: %v", err)
	}

	// Make sure the directory is read-only on Unix systems
	// Note: This doesn't work reliably on Windows
	if err := os.Chmod(readOnlyDir, 0500); err != nil {
		t.Logf("Warning: Could not set directory permissions: %v", err)
	}

	config = Config{
		InputFile:  inputFile,
		OutputFile: filepath.Join(readOnlyDir, "subdir", "output.influx"), // Output to a nonexistent subdir
		Tags:       TagMap{"key": "value"},
	}
	err = processLineProtocol(config)
	if err == nil {
		t.Logf("Note: File permission test might not work on this platform")
	} else if !strings.Contains(err.Error(), "failed to create output file") {
		t.Errorf("Expected 'failed to create output file' error, got: %v", err)
	}
}

// Define the osExit variable for testing
var osExit = os.Exit