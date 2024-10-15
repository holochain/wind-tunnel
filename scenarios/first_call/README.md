## first_call

### Description

This scenario installs a simple app which implements the `init` callback. It will uninstall the app it installed so that 
it can re-install it on the next iteration. This is required to re-run the initialisation callback.

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info cargo run --package first_call -- --connection-string ws://localhost:8888 --duration 300
```
