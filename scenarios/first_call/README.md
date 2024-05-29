## first_call

### Description

This scenario has one:
- **local**: Installs a simple app which implements initialisation callbacks but otherwise doesn't contain a lot of code.

The behaviour will uninstall the app it installed so that it can re-install it on the next iteration. This is required
to re-run the initialisation callback.

### Suggested command

You can run the scenario locally with the following command:

For the `local` case:
```bash
RUST_LOG=info cargo run --package first_call -- --connection-string ws://localhost:8888 --behaviour local --duration 300
```
