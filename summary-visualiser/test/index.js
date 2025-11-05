import child_process from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import url from "node:url";
import test from "tape";
import { visualise } from "../src/index.js";

const __filename = url.fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const windTunnelRootDirName = path.resolve(__dirname, "..", "..");
const summaryVisualiserRootDirName = path.join(windTunnelRootDirName, "summary-visualiser");
const cliPath = path.resolve(summaryVisualiserRootDirName, "cli.js");
console.log(`CLI filename: ${cliPath}`);
const testJsonPath = path.resolve(windTunnelRootDirName, "summariser", "test_data", "3_summary_outputs", "dht_sync_lag-3a1e33ccf661bd873966c539d4d227e703e1496fb54bb999f7be30a3dd493e51.json");
console.log(`Test JSON filename: ${testJsonPath}`);

test("The JS and CSS assets should get cleaned up", (t) => {
    t.plan(1);
    child_process.execSync(`npm -prefix '${summaryVisualiserRootDirName}' run clean`);
    t.notOk(fs.existsSync(path.join(windTunnelRootDirName, "dist")));
});

test("The JS and CSS assets should get built", (t) => {
    t.plan(2);
    child_process.execSync(`npm --prefix '${summaryVisualiserRootDirName}' run build`);
    const fd1 = fs.openSync(path.join(summaryVisualiserRootDirName, "dist", "windTunnel.js"));
    t.ok(fd1, "windTunnel.js should have been built");
    const fd2 = fs.openSync(path.join(summaryVisualiserRootDirName, "dist", "windTunnel.css"));
    t.ok(fd2, "windTunnel.css should have been built");
});

test("The CLI should generate an HTML file when given the proper arguments", (t) => {
    t.plan(2);
    const tmpdir = os.tmpdir();
    const tmpHtmlFilePath = path.join(tmpdir, `wind-tunnel-summary-visualiser-tmp-${crypto.randomBytes(16).toString('hex')}.html`);
    child_process.execSync(`node '${cliPath}' '${testJsonPath}' '${tmpHtmlFilePath}'`);
    const contents = fs.readFileSync(tmpHtmlFilePath, { encoding: "utf-8" });
    t.ok(contents, "temp file should exist and be non-empty");
    t.equal(contents.slice(0, 6), "<html>", "temp file should start with an opening <html> tag");
    fs.unlinkSync(tmpHtmlFilePath);
});

test("The module should generate HTML code when given the proper JSON", (t) => {
    t.plan(3);
    const reportJSON = fs.readFileSync(testJsonPath, { encoding: "utf-8" });
    const reportData = JSON.parse(reportJSON);
    const { html, title } = visualise(reportData);
    t.ok(html, "html should be non-empty");
    t.match(html, /^<section class="scenario scenario-dht-sync-lag">/, "html should start with a section with the class `scenario-dht-sync-lag`");
    t.equal(title, "dht_sync_lag-eZeDqrMBqlqu46953Zs7c", "title should be the scenario name + the run ID");
});