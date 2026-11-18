// Builds latest.json for the Tauri updater from the NSIS bundle produced by
// `tauri build`. Run after a release build: `node scripts/latest-json.mjs`.
import { readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const config = JSON.parse(readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8"));
const version = config.version;
const bundleDir = join(root, "src-tauri/target/release/bundle/nsis");

const installers = readdirSync(bundleDir)
  .filter((name) => name.endsWith("-setup.exe"))
  .map((name) => ({ name, time: statSync(join(bundleDir, name)).mtimeMs }))
  .sort((a, b) => b.time - a.time);

if (!installers.length) {
  throw new Error(`No NSIS installer found in ${bundleDir}`);
}

const installer = installers[0].name;
const signature = readFileSync(join(bundleDir, `${installer}.sig`), "utf8").trim();
const payload = {
  version,
  notes: `Isle ${version}: оновлення в застосунку, одноразовий дозвіл для FPS, плавніші анімації.`,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature,
      url: `https://github.com/k0valyk/isle/releases/download/v${version}/${installer}`,
    },
  },
};

writeFileSync(join(root, "latest.json"), `${JSON.stringify(payload, null, 2)}\n`);
process.stdout.write(`latest.json -> ${installer} (${version})\n`);
