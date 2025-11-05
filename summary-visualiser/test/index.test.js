import child_process from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import url from "node:url";
import { expect, test } from "vitest";
import { visualise } from "../src/index.js";

const __filename = url.fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const windTunnelRootDirName = path.resolve(__dirname, "..", "..");
const summaryVisualiserRootDirName = path.join(windTunnelRootDirName, "summary-visualiser");
const cliPath = path.resolve(summaryVisualiserRootDirName, "cli.js");
const testJsonPath = path.resolve(windTunnelRootDirName, "summariser", "test_data", "3_summary_outputs", "dht_sync_lag-3a1e33ccf661bd873966c539d4d227e703e1496fb54bb999f7be30a3dd493e51.json");

test("The JS and CSS assets should get cleaned up", () => {
    child_process.execSync(`npm -prefix '${summaryVisualiserRootDirName}' run clean`);
    expect(
        fs.existsSync(path.join(windTunnelRootDirName, "dist")),
        "dist directory should have been removed"
    ).toBeFalsy();
});

test("The JS and CSS assets should get built", () => {
    child_process.execSync(`npm --prefix '${summaryVisualiserRootDirName}' run build`);
    const fd1 = fs.openSync(path.join(summaryVisualiserRootDirName, "dist", "windTunnel.js"));
    expect(fd1, "windTunnel.js should have been built").toBeTruthy();
    const fd2 = fs.openSync(path.join(summaryVisualiserRootDirName, "dist", "windTunnel.css"));
    expect(fd2, "windTunnel.css should have been built").toBeTruthy();
});

test("The CLI should generate an HTML file when given the proper arguments", () => {
    const tmpdir = os.tmpdir();
    const tmpHtmlFilePath = path.join(tmpdir, `wind-tunnel-summary-visualiser-tmp-${crypto.randomBytes(16).toString('hex')}.html`);
    child_process.execSync(`node '${cliPath}' '${testJsonPath}' '${tmpHtmlFilePath}'`);
    const contents = fs.readFileSync(tmpHtmlFilePath, { encoding: "utf-8" });
    expect(contents, "temp file should exist and be non-empty").toBeTruthy();
    expect(
        contents.slice(0, 6),
        "temp file should start with an opening <html> tag"
    ).toBe("<html>");
    fs.unlinkSync(tmpHtmlFilePath);
});

test("The module should generate HTML code when given the proper JSON", () => {
    const reportJSON = fs.readFileSync(testJsonPath, { encoding: "utf-8" });
    const reportData = JSON.parse(reportJSON);
    const { html, title } = visualise(reportData);
    expect(html, "html should be non-empty").toBeTruthy();
    expect(
        html,
        "html should start with a section with the class `scenario-dht-sync-lag`"
    ).toMatch(/^<section class="scenario scenario-dht-sync-lag">/);
    expect(title, "title should be the scenario name + the run ID")
        .toBe("dht_sync_lag-eZeDqrMBqlqu46953Zs7c");
});