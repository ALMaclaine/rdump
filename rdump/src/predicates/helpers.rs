use anyhow::{anyhow, Result};
use std::time::{Duration, SystemTime};

pub(super) fn parse_and_compare_size(file_size: u64, query: &str) -> Result<bool> {
    let query = query.trim();
    let (op, size_str) = if query.starts_with(['>', '<', '=']) {
        query.split_at(1)
    } else {
        ("=", query)
    };

    let target_size = size_str
        .trim()
        .to_lowercase()
        .replace("kb", " * 1024")
        .replace('k', " * 1024")
        .replace("mb", " * 1024 * 1024")
        .replace('m', " * 1024 * 1024")
        .replace("gb", " * 1024 * 1024 * 1024")
        .replace('g', " * 1024 * 1024 * 1024")
        .replace('b', "");

    // A simple expression evaluator for "N * N * N..."
    let target_size_bytes = target_size
        .split('*')
        .map(|s| s.trim().parse::<f64>())
        .collect::<Result<Vec<f64>, _>>()?
        .into_iter()
        .product::<f64>() as u64;

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
