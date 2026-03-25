window.createTrendGraph = function (svgId, trendData, meanValue, windowDuration, yUnit) {
    const maxVal = d3.max(trendData);
    const maxYLabel = `${maxVal}${yUnit || ""}`;
    const minYLabel = `0${yUnit || ""}`;
    const widestYLabel = Math.max(maxYLabel.length, minYLabel.length);

    // Keep enough left margin for longer unit strings (e.g. "10000 files/s").
    const margin = {
        top: 16,
        right: 20,
        bottom: 24,
        left: Math.max(60, (widestYLabel + 2) * 7),
    };
    // The width of each data point on the graph.
    // If the graph is large enough, it'll cut the width by 50% or even 80%.
    const pointWidth = trendData.length <= 40 ? 10
                     : trendData.length <= 80 ? 5
                     : 2;
    const width = (trendData.length * pointWidth);
    const height = 120;

    const svg = d3.select(`#${svgId}`)
        .attr("width", width + margin.left + margin.right)
        .attr("height", height + margin.top + margin.bottom)
        // The guts of the graph go into this `<g>` wrapper,
        // in order to make the coordinates of the trend data and axis label easy to work with --
        // they're all zero-referenced to the top-left corner of the graph.
        // It's then moved to the right position in the SVG to make room for the legends.
        .append("g")
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

    // Extract numeric and non-numeric parts of windowDuration.
    const match = windowDuration.match(/^(\d+)(.*)$/);
    const durationValue = parseInt(match[1]);
    const durationUnit = match[2];
    const totalDuration = durationValue * trendData.length;

    svg.append("text")
        .attr("class", "axis-label")
        .attr("x", 0)
        .attr("y", height + 15)
        .attr("text-anchor", "middle")
        .text(`0${durationUnit}`);

    svg.append("text")
        .attr("class", "axis-label")
        .attr("x", width)
        .attr("y", height + 15)
        .attr("text-anchor", "middle")
        .text(`${totalDuration}${durationUnit}`);
};
