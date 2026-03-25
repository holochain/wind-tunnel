window.__trendGraphRegistry = window.__trendGraphRegistry || new Map();
window.__trendGraphRenderQueue = window.__trendGraphRenderQueue || new WeakMap();

function parseWindowDuration(windowDuration) {
    const match = String(windowDuration || "").trim().match(/^([0-9]+(?:\.[0-9]+)?)([a-zA-Z]+)$/);
    if (!match) {
        return {
            durationValue: 0,
            durationUnit: "",
            durationSeconds: 0,
        };
    }

    const durationValue = Number(match[1]);
    const durationUnit = match[2];
    const unitScale = {
        ns: 1e-9,
        us: 1e-6,
        ms: 1e-3,
        s: 1,
        m: 60,
        h: 3600,
        d: 86400,
    };

    return {
        durationValue,
        durationUnit,
        durationSeconds: durationValue * (unitScale[durationUnit] || 1),
    };
}

function getTrendGraphMargin(trendData, yUnit) {
    const maxVal = d3.max(trendData);
    const maxYLabel = `${maxVal}${yUnit || ""}`;
    const minYLabel = `0${yUnit || ""}`;
    const widestYLabel = Math.max(maxYLabel.length, minYLabel.length);

    return {
        top: 16,
        right: 20,
        bottom: 24,
        left: Math.max(60, (widestYLabel + 2) * 7),
    };
}

function renderTrendGraph(entry, plotWidth, margin) {
    const {
        svgElement,
        trendData,
        meanValue,
        windowDuration,
        yUnit,
    } = entry;
    const width = Math.max(1, plotWidth);
    const height = 120;
    const maxVal = d3.max(trendData);
    const maxYLabel = `${maxVal}${yUnit || ""}`;
    const minYLabel = `0${yUnit || ""}`;

    const root = d3.select(svgElement)
        .attr("width", width + margin.left + margin.right)
        .attr("height", height + margin.top + margin.bottom);
    root.selectAll("*").remove();

    // The guts of the graph go into this `<g>` wrapper,
    // in order to make the coordinates of the trend data and axis label easy to work with --
    // they're all zero-referenced to the top-left corner of the graph.
    // It's then moved to the right position in the SVG to make room for the legends.
    const svg = root.append("g")
        .attr("transform", `translate(${margin.left},${margin.top})`);

    // Map time window values to the x range of the graph.
    const x = d3.scaleLinear()
        .domain([0, trendData.length - 1])
        .range([0, width]);

    // Map data point values to the y range of the graph.
    const y = d3.scaleLinear()
        .domain([0, maxVal])
        .range([height, 0]);

    // The trend line.
    const line = d3.line()
        .x((d, i) => x(i))
        .y(d => y(d));
    svg.append("path")
        .datum(trendData)
        .attr("class", "trend-line")
        .attr("d", line);

    // The area under the trend line.
    const area = d3.area()
        .x((d, i) => x(i))
        .y0(height)
        .y1(d => y(d));
    svg.append("path")
        .datum(trendData)
        .attr("class", "trend-area")
        .attr("d", area);

    // Draw the mean line if provided.
    if (meanValue !== null) {
        svg.append("line")
            .attr("class", "mean-line")
            .attr("x1", 0)
            .attr("x2", width)
            .attr("y1", y(meanValue))
            .attr("y2", y(meanValue));
    }

    // Y-axis labels -- always zero on the bottom and the max data point value on the top.
    svg.append("text")
        .attr("class", "axis-label")
        .attr("x", -5)
        .attr("y", 2)
        .attr("text-anchor", "end")
        .attr("dominant-baseline", "hanging")
        .text(maxYLabel);

    svg.append("text")
        .attr("class", "axis-label")
        .attr("x", -5)
        .attr("y", height - 2)
        .attr("text-anchor", "end")
        .attr("dominant-baseline", "text-after-edge")
        .text(minYLabel);

    // X-axis labels -- 0 seconds at the start,
    // and the number of data points × the time window duration at the end.
    const parsedDuration = parseWindowDuration(windowDuration);
    const totalDuration = parsedDuration.durationValue * trendData.length;

    svg.append("text")
        .attr("class", "axis-label")
        .attr("x", 0)
        .attr("y", height + 15)
        .attr("text-anchor", "middle")
        .text(`0${parsedDuration.durationUnit}`);

    svg.append("text")
        .attr("class", "axis-label")
        .attr("x", width)
        .attr("y", height + 15)
        .attr("text-anchor", "middle")
        .text(`${totalDuration}${parsedDuration.durationUnit}`);
}

function renderScenarioTrendGraphs(scenarioElement) {
    const scenarioEntries = [];
    for (const entry of window.__trendGraphRegistry.values()) {
        if (entry.scenarioElement === scenarioElement) {
            scenarioEntries.push(entry);
        }
    }
    if (scenarioEntries.length === 0) {
        return;
    }

    const longestDurationSeconds = Math.max(
        ...scenarioEntries.map(entry => Math.max(entry.totalDurationSeconds, 1e-9)),
    );

    const scenarioMargin = scenarioEntries
        .map(entry => getTrendGraphMargin(entry.trendData, entry.yUnit))
        .reduce((acc, margin) => ({
            top: Math.max(acc.top, margin.top),
            right: Math.max(acc.right, margin.right),
            bottom: Math.max(acc.bottom, margin.bottom),
            left: Math.max(acc.left, margin.left),
        }));

    const availablePlotWidths = scenarioEntries
        .map(entry => {
            const containerWidth = entry.containerElement?.clientWidth || 0;
            if (containerWidth <= 0) {
                return null;
            }
            return Math.max(1, containerWidth - scenarioMargin.left - scenarioMargin.right);
        })
        .filter(width => width !== null);
    if (availablePlotWidths.length === 0) {
        return;
    }

    const targetLongestPlotWidth = Math.min(...availablePlotWidths);
    const pixelsPerSecond = targetLongestPlotWidth / longestDurationSeconds;

    for (const entry of scenarioEntries) {
        const plotWidth = Math.max(1, entry.totalDurationSeconds * pixelsPerSecond);
        renderTrendGraph(entry, plotWidth, scenarioMargin);
    }
}

function scheduleScenarioTrendGraphRender(scenarioElement) {
    if (!scenarioElement) {
        return;
    }
    if (window.__trendGraphRenderQueue.has(scenarioElement)) {
        return;
    }

    const rafHandle = window.requestAnimationFrame(() => {
        window.__trendGraphRenderQueue.delete(scenarioElement);
        renderScenarioTrendGraphs(scenarioElement);
    });
    window.__trendGraphRenderQueue.set(scenarioElement, rafHandle);
}

if (!window.__trendGraphResizeHandlerRegistered) {
    window.addEventListener("resize", () => {
        const scenarios = new Set(
            Array.from(window.__trendGraphRegistry.values()).map(entry => entry.scenarioElement),
        );
        for (const scenarioElement of scenarios) {
            scheduleScenarioTrendGraphRender(scenarioElement);
        }
    });
    window.__trendGraphResizeHandlerRegistered = true;
}

window.createTrendGraph = function (svgId, trendData, meanValue, windowDuration, yUnit) {
    const svgElement = document.getElementById(svgId);
    if (!svgElement) {
        return;
    }

    const parsedDuration = parseWindowDuration(windowDuration);
    const totalDurationSeconds = parsedDuration.durationSeconds * trendData.length;

    window.__trendGraphRegistry.set(svgId, {
        svgElement,
        containerElement: svgElement.closest(".trend-graph"),
        scenarioElement: svgElement.closest(".scenario"),
        trendData,
        meanValue,
        windowDuration,
        yUnit,
        totalDurationSeconds,
    });

    scheduleScenarioTrendGraphRender(svgElement.closest(".scenario"));
};
