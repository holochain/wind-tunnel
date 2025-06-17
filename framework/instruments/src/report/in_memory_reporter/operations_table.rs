use tabled::Tabled;

#[derive(Tabled)]
pub struct OperationRow {
    pub operation_id: String,
    #[tabled(display = "float2")]
    pub avg_time_ms: f64,
    #[tabled(display = "float2")]
    pub min_time_ms: f64,
    #[tabled(display = "float2")]
    pub max_time_ms: f64,
    pub total_operations: usize,
    #[tabled(display = "float2")]
    pub total_duration_ms: f64,
}

fn float2(n: &f64) -> String {
    format!("{:.2}", n)
}
