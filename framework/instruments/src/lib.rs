pub struct OperationRecord {
    operation_id: String,
    started: std::time::Instant,
}

impl OperationRecord {
    pub fn new(operation_id: String) -> Self {
        Self {
            operation_id,
            started: std::time::Instant::now(),
        }
    }
}

pub fn report_operation<T, E>(request_record: OperationRecord, response: &Result<T, E>) {
    let duration = request_record.started.elapsed();
    println!(
        "Operation {} took {}ms, and failed? {:?}",
        request_record.operation_id,
        duration.as_millis(),
        response.is_err(),
    );
}
