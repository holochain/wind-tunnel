window.createTrendGraph = function (svgId, trendData, meanValue, windowDuration, yUnit) {
    // Note: if you change the left margin value,
    // make sure that the CSS for `.no-trend-graph` in `page.html.tmpl`
    // gets updated accordingly.
    const margin = { top: 10, right: 20, bottom: 20, left: 60 };
    const pointWidth = 10; // Width per data point
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

    const maxVal = d3.max(trendData);

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
        .attr("y", 0)
        .attr("text-anchor", "end")
        .attr("alignment-baseline", "middle")
        .text(`${maxVal}${yUnit || ""}`);

    svg.append("text")
        .attr("class", "axis-label")
        .attr("x", -5)
        .attr("y", height)
        .attr("text-anchor", "end")
        .attr("alignment-baseline", "middle")
        .text(`0${yUnit || ""}`);

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
