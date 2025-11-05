/**
 * Round a number to a given precision, adapting to numbers < 1.
 * Examples:
 *
 *   roundAdaptive(1.51631235, 3) -> 1.516
 *   roundAdaptive(0.151631235, 3) -> 0.152
 *   roundAdaptive(0.00000151631235, 3) -> 0.00000152
 *
 * @param {Number} n: the number to round
 * @param {Number} precision: the digits after the decimal place
 *                            OR the first significant digit.
 */
function roundAdaptive(n, precision) {
    if (!n) return n;

    precision = precision || 0;

    const isNegative = n < 0;
    n = Math.abs(n);

    // This loop is guaranteed to end because it would only be infinite if n = 0.
    for (let i = 0; true; i ++) {
        const factor = Math.pow(10, i + precision);
        if (n * Math.pow(10, i) >= 0.1) {
            return Math.round(n * factor) / factor * (isNegative ? -1 : 1);
        }
    }
}

/**
 * Format a number consistently for display,
 * with thousands separators and adaptive rounding.
 *
 * @param {*} n
 * @param {*} precision
 * @returns
 */
function formatNumber(n, precision) {
    return roundAdaptive(n, precision).toLocaleString();
}

export {
    roundAdaptive,
    formatNumber,
};
