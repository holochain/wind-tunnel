import { formatNumber } from "../util.js";

window.createTrendGraph = function (svgId, trendData, meanValue, windowDuration, yUnit) {
    const margin = { top: 25, right: 20, bottom: 20, left: 60 };
    const pointWidth = 40; // Width per data point
    const width = (trendData.length * pointWidth) - margin.left - margin.right;
    const height = 120 - margin.top - margin.bottom;

    const svg = d3.select(`#${svgId}`)
        .attr("width", width + margin.left + margin.right)
        .attr("height", height + margin.top + margin.bottom)
        .append("g")
        .attr("transform", `translate(${margin.left},${margin.top})`);

    const maxVal = d3.max(trendData);
    const range = maxVal;
    const yMax = maxVal + (range * 0.05);

    const x = d3.scaleLinear()
        .domain([0, trendData.length - 1])
        .range([0, width]);

    const y = d3.scaleLinear()
        .domain([0, yMax])
        .range([height, 0]);

    // Create line generator
    const line = d3.line()
        .x((d, i) => x(i))
        .y(d => y(d));

    // Create area generator
    const area = d3.area()
        .x((d, i) => x(i))
        .y0(height)
        .y1(d => y(d));

    // Draw area
    svg.append("path")
        .datum(trendData)
        .attr("class", "trend-area")
        .attr("d", area);

    // Draw line
    svg.append("path")
        .datum(trendData)
        .attr("class", "trend-line")
        .attr("d", line);

    // Draw mean line if provided
    if (meanValue !== null) {
        svg.append("line")
            .attr("class", "mean-line")
            .attr("x1", 0)
            .attr("x2", width)
            .attr("y1", y(meanValue))
            .attr("y2", y(meanValue));
    }

    // Y-axis labels (only top and bottom)
    svg.append("text")
        .attr("class", "axis-label")
        .attr("x", -5)
        .attr("y", 0)
        .attr("text-anchor", "end")
        .attr("alignment-baseline", "middle")
        .text(`${formatNumber(maxVal, 3)}${yUnit || ""}`);

    svg.append("text")
        .attr("class", "axis-label")
        .attr("x", -5)
        .attr("y", height)
        .attr("text-anchor", "end")
        .attr("alignment-baseline", "middle")
        .text(`0${yUnit || ""}`);

    // X-axis labels
    // Extract numeric and non-numeric parts of windowDuration.
    const match = windowDuration.match(/^(\d+)(.*)$/);
    const durationValue = parseInt(match[1]);
    const durationUnit = match[2];
    const totalDuration = durationValue * trendData.length;

    svg.append("text")
        .attr("class", "axis-label")
        .attr("x", 0)
        .attr("y", height + 15)
        .attr("text-anchor", "start")
        .text(`0${durationUnit}`);

    svg.append("text")
        .attr("class", "axis-label")
        .attr("x", width)
        .attr("y", height + 15)
        .attr("text-anchor", "end")
        .text(`${formatNumber(totalDuration, 3)}${durationUnit}`);
};
