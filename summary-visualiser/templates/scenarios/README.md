# Scenario template format

The scenario files in this folder need to be kept in a particular format:

* The file must be named `<scenario_name>.html.tmpl` so the HTML generator tool can match a scenario to a template.
* The HTML markup must follow a particular structure because it will eventually get <!-- TODO: change this to "gets" when this happens --> reused on the holochain.org website, which needs to be able to apply CSS rules predictably. Details in the following section.

## Scenario structure

In order to style the scenarios in a consistent way in different contexts (e.g., standalone pages plus the holochain.org website), there are some classnames that you should use. Here's a basic structure for a scenario called `foo`:

```html
<section class="scenario scenario-foo">
    <div class="scenario-summary"><!-- hint: use the `scenarioSummary` helper -->
        <h1>Foo</h1>
        <div class="scenario-summary-description">
            <p>Test the speed of foo across multiple nodes.</p>
        </div>

        <!-- key/value pairs -->
        <div class="scenario-summary-details">
            <div class="scenario-summary-details-line">
                <div class="scenario-summary-details-label">Label 1</div>
                <div class="scenario-summary-details-value">Value 1</div>
            </div>

            <div class="scenario-summary-details-line">
                <div class="scenario-summary-details-label">Label 2</div>
                <div class="scenario-summary-details-value">Value 2</div>
            </div>
        </div>
    </div>

    <div class="scenario-metrics">
        <!-- A single scalar value -->
        <div class="scenario-metric"><!-- hint use the `scenarioMetric` helper -->
            <div class="scenario-metric-label">
                <div class="scenario-metric-name">Rate of foo</div>
                <div class="scenario-metric-description">A measurement of the number of foos processed per second.</div>
            </div>

            <div class="scenario-metric-content">
                <span class="scalar"><span class="scalar-value">31.624</span><span class="scalar-unit"> foos/s</span></span><!-- hint: use the `scalar` helper -->
            </div>
        </div>

        <!-- Multiple values in one metric -->
        <div class="scenario-metric">
            <div class="scenario-metric-label">
                <div class="scenario-metric-name">Foos per agent</div>
                <div class="scenario-metric-description">A measurement of the number of foos processed per second, broken down by each agent.</div>
            </div>

            <div class="scenario-metric-content">
                <div class="scenario-metric-content-item"><!-- hint: use the `scenarioMetricContentItem` helper -->
                    <div class="scenario-metric-content-item-label">
                        <div class="scenario-metric-content-item-name">Alice</div>
                        <div class="scenario-metric-content-item-description">Agent 1, full arc</div>
                    </div>
                    <div class="scenario-metric-content-item-content">
                        <span class="scalar"><span class="scalar-value">48.246</span><span class="scalar-unit"> foos/s</span></span>
                    </div>
                </div>

                <div class="scenario-metric-content-item">
                    <div class="scenario-metric-content-item-label">
                        <div class="scenario-metric-content-item-name">Bob</div>
                        <div class="scenario-metric-content-item-description">Agent 2, zero arc</div>
                    </div>
                    <div class="scenario-metric-content-item-content">
                        <span class="scalar"><span class="scalar-value">24.118</span><span class="scalar-unit"> foos/s</span></span>
                    </div>
                </div>
            </div>
        </div>
    </div>
</section>
```

That's the basic markup structure of a scenario. You can explore all the helper templates in `templates/helpers/` for other widgets such as mean + std deviation, delta over measured interval, per-agent rates or timings, and a nice trend graph.
