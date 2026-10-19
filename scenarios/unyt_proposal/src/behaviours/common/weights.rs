use std::str::FromStr;

/// Percentage weights to be used on `proposal_actionable`.
///
/// The sum of the weights must sum to `100`.
#[derive(Debug, Clone, Copy)]
pub struct ProposalWeights {
    pub accept: u8,
    pub counter: u8,
    #[allow(dead_code)]
    pub reject: u8,
}

impl ProposalWeights {
    pub fn get_weights_from_env() -> anyhow::Result<Self> {
        let value =
            std::env::var("UNYT_PROPOSAL_WEIGHTS").unwrap_or_else(|_| "60,20,20".to_string());
        ProposalWeights::from_str(&value)
    }
}

impl FromStr for ProposalWeights {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.splitn(3, ',');

        let accept: u8 = split
            .next()
            .ok_or_else(|| anyhow::anyhow!("failed to parse token"))?
            .parse()?;
        let counter: u8 = split
            .next()
            .ok_or_else(|| anyhow::anyhow!("failed to parse token"))?
            .parse()?;
        let reject = split
            .next()
            .ok_or_else(|| anyhow::anyhow!("failed to parse token"))?
            .parse()?;

        if u16::from(accept) + u16::from(counter) + u16::from(reject) != 100 {
            return Err(anyhow::anyhow!("ProposalWeights must sum to 100"));
        }

        Ok(Self {
            accept,
            counter,
            reject,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ProposalWeights;
    use std::str::FromStr;

    #[test]
    fn overflowing_percentage_totals_are_rejected() {
        assert!(ProposalWeights::from_str("255,255,102").is_err());
    }
}
