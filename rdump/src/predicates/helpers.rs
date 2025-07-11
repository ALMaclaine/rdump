use anyhow::{anyhow, Result};
use std::time::{Duration, SystemTime};

pub(super) fn parse_and_compare_size(file_size: u64, query: &str) -> Result<bool> {
    let query = query.trim();
    let (op, size_str) = if query.starts_with(['>', '<', '=']) {
        query.split_at(1)
    } else {
        ("=", query)
    };

    let size_str = size_str.trim().to_lowercase();
    let (num_str, unit) = size_str.split_at(
        size_str
            .find(|c: char| !c.is_digit(10) && c != '.')
            .unwrap_or(size_str.len()),
    );

    let num = num_str.parse::<f64>()?;
    let multiplier = match unit.trim() {
        "b" | "" => 1.0,
        "kb" | "k" => 1024.0,
        "mb" | "m" => 1024.0 * 1024.0,
        "gb" | "g" => 1024.0 * 1024.0 * 1024.0,
        _ => return Err(anyhow!("Invalid size unit: {}", unit)),
    };

    let target_size_bytes = (num * multiplier) as u64;

    match op {
        ">" => Ok(file_size > target_size_bytes),
        "<" => Ok(file_size < target_size_bytes),
        "=" => Ok(file_size == target_size_bytes),
        _ => Err(anyhow!("Invalid size operator: {}", op)),
    }
}

pub(super) fn parse_and_compare_time(modified_time: SystemTime, query: &str) -> Result<bool> {
    let now = SystemTime::now();
    let (op, duration_str) = query.split_at(1);
    let duration_str = duration_str.trim();

    let duration_secs = if let Some(num_str) = duration_str.strip_suffix('s') {
        num_str.parse::<u64>()?
    } else if let Some(num_str) = duration_str.strip_suffix('m') {
        num_str.parse::<u64>()? * 60
    } else if let Some(num_str) = duration_str.strip_suffix('h') {
        num_str.parse::<u64>()? * 3600
    } else if let Some(num_str) = duration_str.strip_suffix('d') {
        num_str.parse::<u64>()? * 86400
    } else {
        return Err(anyhow!("Invalid time unit in '{}'", query));
    };

    let duration = Duration::from_secs(duration_secs);
    let threshold_time = now
        .checked_sub(duration)
        .ok_or(anyhow!("Time calculation underflow"))?;

    match op {
        ">" => Ok(modified_time > threshold_time), // Modified more recently than
        "<" => Ok(modified_time < threshold_time), // Modified longer ago than
        _ => Err(anyhow!("Invalid time operator: {}", op)),
    }
}
