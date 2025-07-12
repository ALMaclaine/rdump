use anyhow::{anyhow, Result};
use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
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
    let (op, time_str) = if query.starts_with(['>', '<', '=']) {
        query.split_at(1)
    } else {
        ("=", query)
    };
    let time_str = time_str.trim();

    let threshold_time = if let Ok(duration) = parse_relative_time(time_str) {
        now.checked_sub(duration)
            .ok_or_else(|| anyhow!("Time calculation underflow"))?
    } else if let Ok(datetime) = parse_absolute_time(time_str) {
        datetime
    } else {
        return Err(anyhow!("Invalid date format: '{}'", time_str));
    };

    match op {
        ">" => Ok(modified_time > threshold_time),
        "<" => Ok(modified_time < threshold_time),
        "=" => {
            // For date-only comparisons, check if the modified time is within the same day
            if time_str.len() == 10 {
                let modified_local = chrono::DateTime::<Local>::from(modified_time);
                let threshold_local = chrono::DateTime::<Local>::from(threshold_time);
                Ok(modified_local.date_naive() == threshold_local.date_naive())
            } else {
                Ok(modified_time == threshold_time)
            }
        }
        _ => Err(anyhow!("Invalid time operator: {}", op)),
    }
}

fn parse_relative_time(time_str: &str) -> Result<Duration> {
    let (num_str, unit) = time_str.split_at(
        time_str
            .find(|c: char| !c.is_digit(10))
            .unwrap_or(time_str.len()),
    );
    let num = num_str.parse::<u64>()?;
    let multiplier = match unit.trim() {
        "s" => 1,
        "m" => 60,
        "h" => 3600,
        "d" => 86400,
        "w" => 86400 * 7,
        "y" => 86400 * 365,
        _ => return Err(anyhow!("Invalid time unit")),
    };
    Ok(Duration::from_secs(num * multiplier))
}

fn parse_absolute_time(time_str: &str) -> Result<SystemTime> {
    let datetime = if let Ok(dt) = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S") {
        dt
    } else if let Ok(date) = NaiveDate::parse_from_str(time_str, "%Y-%m-%d") {
        date.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
    } else {
        return Err(anyhow!("Invalid absolute date format"));
    };

    Ok(Local
        .from_local_datetime(&datetime)
        .single()
        .ok_or_else(|| anyhow!("Failed to convert to local time"))?
        .into())
}
