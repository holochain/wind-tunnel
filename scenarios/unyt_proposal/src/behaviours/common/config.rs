use super::weights::ProposalWeights;

/// All environment-derived configuration for the proposal scenario.
#[derive(Debug, Clone, Copy)]
pub struct ProposalConfig {
    /// How incoming proposals are split into accept/counter/reject (`UNYT_PROPOSAL_WEIGHTS`).
    pub weights: ProposalWeights,
    /// Percentage of the spendable amount to spend per round (`UNYT_SPEND_FRACTION_PCT`).
    pub spend_fraction_pct: u8,
    /// Percentage by which counter-proposal amounts are reduced (`UNYT_COUNTER_ADJUSTMENT_PCT`).
    pub counter_adjustment_pct: u8,
    /// Number of negotiation rounds after which a proposal is force-accepted
    /// (`UNYT_MAX_NEGOTIATION_ROUNDS`).
    pub max_negotiation_rounds: usize,
    /// Percentage of incoming commitments to accept (`UNYT_COMMITMENT_ACCEPT_PCT`).
    pub commitment_accept_pct: u8,
}

impl ProposalConfig {
    /// Read the configuration from the environment, applying defaults and validating ranges.
    pub fn from_env() -> anyhow::Result<Self> {
        let weights = ProposalWeights::get_weights_from_env()?;
        let spend_fraction_pct = parse_pct_env("UNYT_SPEND_FRACTION_PCT", 10)?;
        let counter_adjustment_pct = parse_pct_env("UNYT_COUNTER_ADJUSTMENT_PCT", 10)?;
        let commitment_accept_pct = parse_pct_env("UNYT_COMMITMENT_ACCEPT_PCT", 80)?;
        let max_negotiation_rounds: usize = std::env::var("UNYT_MAX_NEGOTIATION_ROUNDS")
            .unwrap_or_else(|_| "5".to_string())
            .parse()?;

        Ok(Self {
            weights,
            spend_fraction_pct,
            counter_adjustment_pct,
            max_negotiation_rounds,
            commitment_accept_pct,
        })
    }
}

/// Read a percentage env var, falling back to `default`, and validate it is between 0 and 100.
fn parse_pct_env(name: &str, default: u8) -> anyhow::Result<u8> {
    let value: u8 = std::env::var(name)
        .unwrap_or_else(|_| default.to_string())
        .parse()?;
    if value > 100 {
        anyhow::bail!("{name} must be between 0 and 100, got {value}");
    }
    Ok(value)
}
