#!/usr/bin/env node

import fs from "node:fs";
import { readFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

import { Command } from "commander";
import handlebars from "handlebars";
import { visualise } from "./src/index.js";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const program = new Command();
program
    .name("wind-tunnel-summary-visualiser")
    .description("A tool to generate a pretty HTML report from Wind Tunnel scenario run summary JSON.")
    .version("0.0.1")
    .option("-t, --template <template>", "HTML page template to insert the generated HTML into.", "page.hbs")
    .argument("<inputFile>", "The path to the input JSON.")
    .argument("<outputFile>", "the path to the HTML file you want to create.");

program.parse();

const { template } = program.opts();
const [ inputFile, outputFile ] = program.args;

// Load the JSON.
let json;
try {
    json = fs.readFileSync(path.resolve(inputFile));
} catch (e) {
    console.error(`Couldn't read JSON. Error: "${e.message}"`);
    process.exit(1);
}

// Try to parse it.
let scenarios;
try {
    scenarios = JSON.parse(json);
} catch (e) {
    console.error(`couldn't parse JSON. Error: "${e.message}"`);
    process.exit(1);
}

// Generate the HTML and title.
let report;
try {
    report = visualise(scenarios);
} catch (e) {
    console.error(e.message);
    process.exit(1);
}

// Load the JS and CSS assets.
let js;
try {
    js = fs.readFileSync(path.join(__dirname, "dist", "windTunnel.js"), { encoding: "utf-8" });
} catch (e) {
    console.error(`Couldn't load the required JavaScript for embedding in the page. Did you run \`npm run build\` yet? Error message: "${e.message}"`);
    process.exit(1);
}
let css;
try {
    css = fs.readFileSync(path.join(__dirname, "dist", "windTunnel.css"), { encoding: "utf-8" });
} catch (e) {
    console.error(`Couldn't load the basic CSS. Error message: "${e.message}"`);
    process.exit(1);
}

// Load and compile the page template.
let pageTemplate;
try {
    // Register partials for the CSS and JS; this helps them be indented properly.
    handlebars.registerPartial("js", js);
    handlebars.registerPartial("css", css);
    pageTemplate = handlebars.compile(fs.readFileSync(path.join(__dirname, template), { encoding: 'utf8' }));
} catch (e) {
    console.error(`Couldn't read page template ${template}. Error message: "${e.message}"`);
    process.exit(1);
}

// Put the HTML, title, and assets into the page template.
let page;
try {
    page = pageTemplate(report);
} catch (e) {
    console.error(`Couldn't build HTML page. Error message: "${e.message}"`);
    process.exit(1);
}

// Write the file.
try {
    fs.writeFileSync(path.resolve(outputFile), page, { encoding: "utf-8" });
} catch (e) {
    console.error(`Couldn't save HTML page to \`${outputFile}\`. Error message: "${e.message}"`);
    process.exit(1);
}
process.exit();
