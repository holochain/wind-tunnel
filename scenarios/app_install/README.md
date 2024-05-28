## app_install

### Description

This scenario has two roles:
- **minimal**: Installs a simple app which implements initialisation callbacks but otherwise doesn't contain a lot of code.
- **large**: Installs a larger app that contains some dependencies and some generated data as padding to make the bundle larger.
  These are intended to catch a compilation slowdown or issues with copying around large WASMs (e.g. accidental cloning or tracing).

In each case, the behaviour will uninstall the app it installed so that it can re-install it on the next iteration.

### Suggested command

You can run the scenario locally with the following command:

For the `minimal` case:
```bash
RUST_LOG=info cargo run --package app_install -- --connection-string ws://localhost:8888 --behaviour minimal --duration 300
```

For the `large` case:
```bash
RUST_LOG=info cargo run --package app_install -- --connection-string ws://localhost:8888 --behaviour large --duration 300
```
