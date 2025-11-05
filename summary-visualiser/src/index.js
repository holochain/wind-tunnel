import fs from "node:fs";
import path from "node:path";
import handlebars from "handlebars";
import scenarioTransforms from "./scenarioTransforms.js";
import { fileURLToPath } from "node:url";
import { formatNumber } from "./util.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const templatesFolderPath = path.join(__dirname, "templates");
const partialsFolderPath = path.join(templatesFolderPath, "partials");

// Load all the partials and make them available to the template.
const partials = fs.readdirSync(partialsFolderPath);
for (const p of partials) {
    const partialName = p.replace(".hbs", "");
    const partialContent = fs.readFileSync(path.join(partialsFolderPath, p), { encoding: "utf8" });
    handlebars.registerPartial(partialName, partialContent);
}

// A few helpers
handlebars.registerHelper("datetime", (ts) => `${(new Date(ts * 1000)).toLocaleString()} (${Intl.DateTimeFormat().resolvedOptions().timeZone})`);
handlebars.registerHelper("json", (data) => JSON.stringify(data));
handlebars.registerHelper("number", function(n) {
    // There's always an options object passed to helpers,
    // so we know that a precision has been passed if there are three args.
    const precision = arguments.length > 2 ? arguments[1] : 0;
    return formatNumber(n, precision);
});
handlebars.registerHelper("iflt", function(a, b, options) { return (a < b) ? options.fn(this) : options.inverse(this); });
handlebars.registerHelper("ifgt", function(a, b, options) { return (a > b) ? options.fn(this) : options.inverse(this); });
handlebars.registerHelper("ifeq", function(a, b, options) { return (a == b) ? options.fn(this) : options.inverse(this); });
handlebars.registerHelper("percentChange", (a, b) => {
    if (a == b) {
        return "0%";
    }
    if (!a || !b) {
        return "n/a";
    }
    return `${formatNumber(((b - a) / a) * 100)}%`;
});
handlebars.registerHelper("plural", function(n, singular, plural) {
    const precision = arguments.length > 4 ? arguments[3] : 0;
    return n == 1 ? `${formatNumber(n, precision)}${singular}` : `${formatNumber(n, precision)}${plural}`;
});

function visualise(scenarios) {
    let scenarioHtmls = [];
    let scenarioTitles = [];

    if (!Array.isArray(scenarios)) {
        scenarios = [scenarios];
    }

    for (let scenario of scenarios) {
        const scenarioName = scenario.run_summary.scenario_name;
        // Is there a specific transform for this scenario?
        // If not, just create a generic one that adds a title and blank description.
        const transform = scenarioTransforms[scenarioName] || ((s) => ({ ...s, title: scenarioName, description: null }));
        // Modify the scenario as required.
        scenario = transform(scenario);

        // Load the scenario template.
        let templateContent;
        try {
            templateContent = fs.readFileSync(path.join(templatesFolderPath, "scenarios", `${scenarioName}.hbs`), { encoding: 'utf8' });
        } catch (e) {
            throw new Error(`Couldn't load template for ${scenarioName} scenario. Error message: "${e.message}"`);
        }

        // Now parse it.
        let template;
        try {
            template = handlebars.compile(templateContent);
        } catch (e) {
            throw new Error(`Couldn't compile template for ${scenarioName} scenario. Error message: "${e.message}"`);
        }

        // And generate the HTML.
        try {
            scenarioHtmls.push(template(scenario));
        } catch (e) {
            throw new Error(`Couldn't generate HTML for ${scenarioName} scenario. Error message: "${e.message}"`);
        }

        // We'll eventually construct a title from all the scenarios in the array.
        scenarioTitles.push(`${scenario.run_summary.scenario_name}-${scenario.run_summary.run_id}`)
    }

    return {
        html: scenarioHtmls.join("\n\n"),
        title: scenarioTitles.join(", "),
    };
}

export { visualise };