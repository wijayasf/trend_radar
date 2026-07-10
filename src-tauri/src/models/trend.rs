#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sentiment {
    Positive,
    Negative,
    Neutral,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CostSignal {
    Expensive,
    TokenHeavy,
    QuotaLimited,
    WorthIt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrendTopic {
    pub name: String,
    pub region: TrendRegion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrendRegion {
    Indonesia,
    Global,
}
