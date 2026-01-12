use adif_parser::parse_adi;
use regex::Regex;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::process;

const CWOPS_ROSTER_URL: &str = "https://docs.google.com/spreadsheets/d/e/2PACX-1vRF-m4jfF0bPP-UglTk0Le0bdwB16q_tfDoiotBjhmCsQWYvO-tPA22w9ea18RAAyB9-0ZFLmDAkTqg/pubhtml?gid=1402724567&single=true&widget=false&headers=false";

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut check_cwops = false;
    let mut filename: Option<&str> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--cwops" | "-c" => check_cwops = true,
            arg if !arg.starts_with('-') => filename = Some(arg),
            other => {
                eprintln!("Unknown option: {}", other);
                print_usage(&args[0]);
                process::exit(1);
            }
        }
        i += 1;
    }

    let filename = match filename {
        Some(f) => f,
        None => {
            print_usage(&args[0]);
            process::exit(1);
        }
    };

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

    // Collect unique callsigns
    let mut unique_calls: HashSet<String> = HashSet::new();
    for record in &adif.records {
        if let Some(call) = record.call() {
            unique_calls.insert(call.to_uppercase());
        }
    }

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

    // CWops membership analysis
    if check_cwops {
        println!();
        println!("CWops Membership Analysis");
        println!("=========================");

        match fetch_cwops_roster() {
            Ok(roster) => {
                let mut member_count = 0;
                let mut non_members: Vec<String> = Vec::new();

                for call in &unique_calls {
                    if is_cwops_member(call, &roster) {
                        member_count += 1;
                    } else {
                        non_members.push(call.clone());
                    }
                }

                non_members.sort();

                println!("Unique CWops members worked: {}", member_count);
                println!("Non-members worked: {}", non_members.len());

                if !non_members.is_empty() {
                    println!("Non-member callsigns:");
                    for call in &non_members {
                        println!("  {}", call);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error fetching CWops roster: {}", e);
                process::exit(1);
            }
        }
    }
}

fn print_usage(program: &str) {
    eprintln!("Usage: {} [OPTIONS] <adif_file>", program);
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -c, --cwops    Check contacts against CWops member roster");
}

/// Check if a callsign matches a CWops member, handling / prefixes and suffixes
/// e.g., "K9DX/3" should match "K9DX", and "VE3/K9DX" should also match "K9DX"
fn is_cwops_member(call: &str, roster: &HashSet<String>) -> bool {
    // Direct match
    if roster.contains(call) {
        return true;
    }

    // Check each part split by "/" against the roster
    for part in call.split('/') {
        if roster.contains(part) {
            return true;
        }
    }

    false
}

/// Fetch and parse the CWops member roster
fn fetch_cwops_roster() -> Result<HashSet<String>, String> {
    eprintln!("Fetching CWops roster...");

    let response = ureq::get(CWOPS_ROSTER_URL)
        .call()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let html = response
        .into_string()
        .map_err(|e| format!("Failed to read response: {}", e))?;

    // Extract callsigns from <td class="s19"...>CALLSIGN</td> elements
    let re = Regex::new(r#"<td class="s19"[^>]*>([A-Z0-9/]+)</td>"#).unwrap();

    let mut roster: HashSet<String> = HashSet::new();

    for cap in re.captures_iter(&html) {
        if let Some(call) = cap.get(1) {
            roster.insert(call.as_str().to_uppercase());
        }
    }

    eprintln!("Loaded {} CWops members", roster.len());

    Ok(roster)
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
