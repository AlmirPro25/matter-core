# Node Native Host Example

This example loads the `matter-bridge-nodejs-native` N-API addon in a real Node.js host and validates exported functions.

```powershell
cargo build -p matter-bridge-nodejs-native
copy F:\Users\almir\Desktop\matter_target\debug\matter_bridge_nodejs_native.dll F:\Users\almir\Desktop\matter_target\debug\matter_bridge_nodejs_native.node
node examples\node_native_host\smoke.js F:\Users\almir\Desktop\matter_target\debug\matter_bridge_nodejs_native.node
powershell -ExecutionPolicy Bypass -File .\scripts\native-ffi-smoke.ps1
```

Validated exports:

- `matterBridgeInit()`
- `matterBridgeVersion()`
- `matterBridgeAddIntsJson(argsJson)`

The smoke copies the Windows `.dll` to `.node` before loading it, because Node expects native addons to use the `.node` extension.
