const addonPath = process.argv[2];

if (!addonPath) {
  console.error("usage: node examples/node_native_host/smoke.js <addon.node>");
  process.exit(1);
}

const addon = require(addonPath);
const added = JSON.parse(
  addon.matterBridgeAddIntsJson(
    JSON.stringify([
      { type: "int", value: 40 },
      { type: "int", value: 2 },
    ]),
  ),
);

const out = {
  init: addon.matterBridgeInit(),
  version: addon.matterBridgeVersion(),
  keys: Object.keys(addon).sort(),
  added,
};

console.log(JSON.stringify(out));

if (out.init !== "Matter Node.js Native Bridge initialized") {
  process.exit(2);
}

if (out.version !== "2.1.0") {
  process.exit(3);
}

if (out.added.type !== "int" || out.added.value !== 42) {
  process.exit(4);
}
