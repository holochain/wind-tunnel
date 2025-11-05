function shrinkIdentifier(identifier) {
    return `${identifier.slice(3, 6)}...${identifier.slice(-3)}`;
}

function transformDbUtilisationMetric(metric) {
    return {
        // These values are database utilisation ratios;
        // turn them into percentages.
        mean: metric.mean * 100,
        max: metric.max * 100,
        // We don't need min, std, or count.
    };
}

function transformAuthoredDbUtilisation(dbUtilisation) {
    return Object.entries(dbUtilisation)
        .map(([k, v]) => ({
            name: `Utilisation for DNA ${shrinkIdentifier(k.match(/(?<=DnaHash\()([^)]+)(?=\))/)[0])} / agent ${shrinkIdentifier(k.match(/(?<=AgentPubKey\()([^)]+)(?=\))/)[0])}`,
            ...transformDbUtilisationMetric(v),
        }));
}

export default {
    "dht_sync_lag": (s) => {
        return {
            ...s,
            scenario_metrics: {
                ...s.scenario_metrics,
                authored_db_utilization: transformAuthoredDbUtilisation(s.scenario_metrics.authored_db_utilization),
                conductor_db_utilization: transformDbUtilisationMetric(s.scenario_metrics.conductor_db_utilization),
                dht_db_utilization: transformDbUtilisationMetric(s.scenario_metrics.dht_db_utilization),
            },
            title: "DHT Sync Lag",
            description: "TODO -- needs description",
        };
    },
}
