use adif_parser::parse_adi;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <adif_file>", args[0]);
        process::exit(1);
    }

    let filename = &args[1];
    let content = match fs::read_to_string(filename) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", filename, e);
            process::exit(1);
        }
    };

    let adif = match parse_adi(&content) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error parsing ADIF file: {:?}", e);
            process::exit(1);
        }
    };

    let total_qsos = adif.records.len();

    // Find dupes: same callsign on same band
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut dupe_count = 0;

    for record in &adif.records {
        let call = record.call().unwrap_or("").to_uppercase();
        let band = record.band().unwrap_or("").to_uppercase();

        let key = (call, band);
        if seen.contains(&key) {
            dupe_count += 1;
        } else {
            seen.insert(key);
        }
    }

    // Find 10-minute window with most contacts
    // Parse timestamps and group by 10-minute windows
    let mut timestamps: Vec<u64> = Vec::new();

    for record in &adif.records {
        if let (Some(date), Some(time)) = (record.qso_date(), record.time_on()) {
            if let Some(ts) = parse_datetime(date, time) {
                timestamps.push(ts);
            }
        }
    }

    timestamps.sort();

    let (best_window_start, best_count) = find_best_10min_window(&timestamps);

    println!("ADIF Log Analysis");
    println!("=================");
    println!("Total QSOs: {}", total_qsos);
    println!("Duplicate QSOs (same call/band): {}", dupe_count);

    if best_count > 0 {
        let (date_str, time_str) = format_timestamp(best_window_start);
        println!(
            "Best 10-minute window: {} {} UTC with {} QSOs",
            date_str, time_str, best_count
        );
    } else {
        println!("Best 10-minute window: No valid timestamps found");
    }
}

/// Parse ADIF date (YYYYMMDD) and time (HHMMSS or HHMM) into minutes since epoch
fn parse_datetime(date: &str, time: &str) -> Option<u64> {
    if date.len() != 8 {
        return None;
    }

    let year: u64 = date[0..4].parse().ok()?;
    let month: u64 = date[4..6].parse().ok()?;
    let day: u64 = date[6..8].parse().ok()?;

    let hour: u64 = time.get(0..2)?.parse().ok()?;
    let minute: u64 = time.get(2..4)?.parse().ok()?;

    // Convert to minutes since a reference point (simplified calculation)
    // We don't need exact epoch time, just relative ordering
    let days = year * 365 + month * 31 + day;
    let minutes = days * 24 * 60 + hour * 60 + minute;

    Some(minutes)
}

/// Format timestamp back to date and time strings
fn format_timestamp(minutes: u64) -> (String, String) {
    let total_minutes = minutes;
    let minute_of_day = total_minutes % (24 * 60);
    let hour = minute_of_day / 60;
    let minute = minute_of_day % 60;

    let days = total_minutes / (24 * 60);
    let year = days / 365;
    let remaining_days = days % 365;
    let month = remaining_days / 31;
    let day = remaining_days % 31;

    let date_str = format!("{:04}-{:02}-{:02}", year, month, day);
    let time_str = format!("{:02}:{:02}", hour, minute);

    (date_str, time_str)
}

/// Find the 10-minute window with the most contacts using sliding window
fn find_best_10min_window(sorted_timestamps: &[u64]) -> (u64, usize) {
    if sorted_timestamps.is_empty() {
        return (0, 0);
    }

    let mut best_start = sorted_timestamps[0];
    let mut best_count = 0;

    // Use sliding window approach
    let mut left = 0;

    for right in 0..sorted_timestamps.len() {
        // Move left pointer to maintain 10-minute window
        while sorted_timestamps[right] - sorted_timestamps[left] >= 10 {
            left += 1;
        }

        let count = right - left + 1;
        if count > best_count {
            best_count = count;
            best_start = sorted_timestamps[left];
        }
    }

    (best_start, best_count)
}
