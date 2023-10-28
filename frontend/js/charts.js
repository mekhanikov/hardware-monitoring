class Retriever {
    constructor(source, ts) {
        this.source = source;
        this.timestampOfFirstRecord = ts;
    }

    async getSize() {
        let xhr = await makeRequest("HEAD", this.source);
        let s = parseInt(xhr.getResponseHeader("Content-Length"))
        return s;
    }

    async retrieveScieenceOfffset(offset) {
        let data = await this.retrieveExact(this.source, offset);
        return data;
    }

    async retrieveLastN(n) {
        let size = await this.getSize();
        let countToReturn = (size > n) ? n : size;
        return await this.retrieveExact(this.source, size - countToReturn, size);
    }

    toDate(unixTimestamp) {
        return new Date(
            unixTimestamp * 1000
        )
    }

    async retrieveExact(url, offset, countToReturn) {
        var data = await makeRequest("GET", url, offset, countToReturn);
        var res = [];
        let that = this;
        data = data.response;
        data = new Uint8Array(data)
        data.forEach(function (c, i) {
            let timestamp = that.timestampOfFirstRecord + offset + i;
            let value = c;
            res.push({ date: that.toDate(timestamp), value: value });
        });
        return res
    }
}

class Chart {

    dataRetrievers = []
    lineChart;
    lastTick;
    lastDataOffset = 0;
    collectedData = [];

    constructor(url) {
        this.url = url;
    }
}

export const lineChart = (data, {
    svgId = 'line-chart',
    x = (x) => x,
    y = (y) => y,
    defined, // for gaps in data
    curve = d3.curveNatural, // method of interpolation between points (curveNatural, curveLinear)
    marginTop = 20, // top margin, in pixels
    marginRight = 30, // right margin, in pixels
    marginBottom = 30, // bottom margin, in pixels
    marginLeft = 40, // left margin, in pixels
    width = 640, // outer width, in pixels
    height = 400, // outer height, in pixels
    xType = d3.scaleTime, // the x-scale type
    xDomain, // [xmin, xmax]
    xRange = [marginLeft, width - marginRight], // [left, right]
    yType = d3.scaleLinear, // the y-scale type
    yDomain, // [ymin, ymax]
    yRange = [height - marginBottom, marginTop], // [bottom, top]
    yFormat, // a format specifier string for the y-axis
    yLabel, // a label for the y-axis
    color = 'currentColor', // stroke color of line
    strokeLinecap = 'round', // stroke line cap of the line
    strokeLinejoin = 'round', // stroke line join of the line
    strokeWidth = 1.5, // stroke width of line, in pixels
    strokeOpacity = 0.5, // stroke opacity of line
    count = 1, // graphs
} = {}) => {
    if (defined === undefined) defined = (d) => !isNaN(x(d)) && !isNaN(y(d));
    if (xDomain === undefined) xDomain = () => {
        if ((data && data[0].length > 0)) {
            let ts = x(data[0][data[0].length-1]);
            let d1 = x(data[0][data[0].length - 1]);
            let d2 = x(data[0][data[0].length - 100]);
            return [d2, d1]
        } else {
            return [-100, 0];
        }
    }
    if (yDomain === undefined) yDomain = [0, 100];

    let xScale = () => xType(xDomain(), xRange);
    var yScale = yType(yDomain, yRange);

    let xAxis = () => { return d3.axisBottom(xScale()).ticks(width / 80).tickSizeOuter(0); };
    var yAxis = d3.axisLeft(yScale).ticks(height / 40, yFormat);

    const line = () => {
        return d3.line()
            .defined(d => defined(d))
            .curve(curve)
            .x(d => xScale()(x(d)))
            .y(d => yScale(y(d)))
    }


    const svg = d3.create('svg')
        .attr('id', svgId)
        .attr('width', width+"%")
        .attr('height', height)
        .attr('viewBox', [0, 0, width, height])
        .attr('style', 'max-width: 100%; height: auto; height: intrinsic;');

    const clip = svgId + '_clip';
    svg.append('clipPath')
        .attr('id', clip)
        .append('rect')
        .attr('x', marginLeft)
        .attr('y', marginTop)
        .attr('width', width - marginLeft - marginRight)
        .attr('height', height - marginTop - marginBottom);

    let gx = svg.append('g')
        .attr('id', svgId + "_xAxis")
        .attr('transform', `translate(0,${height - marginBottom})`);

    svg.append('g')
        .attr('id', svgId + "_yAxis")
        .attr('transform', `translate(${marginLeft},0)`)
        .call(yAxis)
        .call(g => g.select('.domain').remove())
        .call(g => g.selectAll('.tick line').clone()
            .attr('x2', width - marginLeft - marginRight)
            .attr('stroke-opacity', 0.1))
        .call(g => g.append('text')
            .attr('x', -marginLeft)
            .attr('y', 10)
            .attr('fill', 'currentColor')
            .attr('text-anchor', 'start')
            .text(yLabel));

    for (var i = 0; i < count; i++) {
        svg.append('path')
            .attr('id', svgId + "_path" + i)
            .attr('fill', 'none')
            .attr('stroke', color[i])
            .attr('stroke-width', strokeWidth)
            .attr('stroke-linecap', strokeLinecap)
            .attr('stroke-linejoin', strokeLinejoin)
            .attr('stroke-opacity', strokeOpacity);
    }

    async function appendValue(valToAppend) {
        data = valToAppend;
        svg.select('g#' + svgId + "_xAxis")
            .call(xAxis());
        svg.select('g#' + svgId + "_yAxis")
            .call(yAxis);
        for (var i = 0; i < count; i++) {
            svg.select('path#' + svgId + "_path" + i)
                .attr('clip-path', `url(#${clip})`)
                .attr('d', line()(data[i]));
        }
    }

    let n = svg.node();
    n.appendValue = appendValue;

    return n;
}

export const makeRequest = async (method, url, start, end) => {
    // async makeRequest(method, url, start, end) {
    return new Promise(function (resolve, reject) {
        let xhr = new XMLHttpRequest();
        xhr.open(method, url, true);
        xhr.responseType = "arraybuffer";
        if (start) {
            if (!end) {end=""}
            xhr.setRequestHeader('Range', `bytes=${start}-${end}`);
        }

        xhr.onload = function () {
            if (this.status >= 200 && this.status < 300) {
                // resolve(xhr.response);
                resolve(xhr);
            } else {
                reject({
                    status: this.status,
                    statusText: xhr.statusText
                });
            }
        };
        xhr.onerror = function () {
            reject({
                status: this.status,
                statusText: xhr.statusText
            });
        };
        xhr.send();
    });
}

export const retrieveMaster = async (url) => {
    var data = await makeRequest("GET", url);
    data = data.response;
    data = new Uint32Array(data)
    return data
}

class Sheet {
    constructor(charts, retrievers, collectedData) {
        this.charts = charts;
        this.retrievers = retrievers;
        this.collectedData = collectedData;
    }

    async nextTick() {
        let now = Math.floor(Date.now() / 1000);
        var incremental = true;
        if (this.lastTick) {
            let delta = now - this.lastTick;
            if (delta > 100) {
                incremental = false;
            }
        } else {
            incremental = false;
        }
        if (!incremental) {
            this.lastDataOffset = await this.retrievers[0].getSize();
        }
        let dataFutures = incremental ?
            this.retrievers.map(retriever => retriever.retrieveScieenceOfffset(this.lastDataOffset)) :
            this.retrievers.map(retriever => retriever.retrieveLastN(100));
        let data = []
        data = await Promise.all(dataFutures)
        .catch((error) => {
            console.error(error.message);
        });
        if (!data || data.length<=0) return;
        var min = Infinity;
        for (var i = 0; i < data.length; i++) {
            let size = data[i].length
            if (size < min) {
                min = size;
            }
        }
        if (incremental) {
             for (var i = 0; i < data.length; i++) {
                 this.collectedData[i] = this.collectedData[i].concat(data[i].slice(0, min));
                     if (this.collectedData[i].length > 100) {
                         this.collectedData[i] = this.collectedData[i].slice(-100);
                     }
             }
             this.lastDataOffset += min;
        } else {
            for (var i = 0; i < data.length; i++) {
                this.collectedData[i] = data[i].slice(0, min);
            }
        }
        let sourceToDataMap = new Map();
        let that = this;
        this.retrievers.forEach(function (retriever, i) {
            sourceToDataMap.set(retriever.source, that.collectedData[i]);
        });
        let dataPerChart = [];
        this.charts.forEach(function (chart) {
            let dataChart = [];
            chart.sources.forEach(function (chartSource) {
                let d = sourceToDataMap.get(chartSource);
                dataChart.push(d);
            });
            chart.lineChart.appendValue(dataChart)
        });
        this.lastTick = now;
    }
}

export const buildChart = async () => {


    let master = await retrieveMaster("/start.bin")
    let len = master.length
    var lastRecordOffset = len - len % 2;
    let ts = master[lastRecordOffset]
    console.log("ts: " + ts)

    let sources = [
        `/${ts}-cpu-0.bin`,
        `/${ts}-cpu-1.bin`,
    ];

    let retrievers = sources.map(it => new Retriever(it, ts));
    let collectedData = [];
    for (var i = 0; i < retrievers.length; i++) {
        collectedData.push([]);
    }

    let chart = new Chart();
    const lineChart0 = lineChart([], {
        x: d => d.date,
        y: d => d.value,
        yLabel: 'CPU Usage, %',
        width: 400,
        height: 250,
        color: ["red", "blue", "green", "orange"],
        count: 4
    });
    chart.lineChart = lineChart0;
    chart.sources = [
        `/${ts}-cpu-0.bin`,
        `/${ts}-cpu-1.bin`,
    ]

    let charts = []
    charts.push(chart)
    return new Sheet(charts, retrievers, collectedData);
}
