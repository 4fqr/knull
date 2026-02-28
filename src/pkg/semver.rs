//! Semver Resolution Module
//! Handles semantic versioning constraints and comparison

use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    Exact(String),
    Caret(String), // ^1.0.0
    Tilde(String), // ~1.0.0
    GreaterThan(String),
    GreaterThanOrEqual(String),
    LessThan(String),
    LessThanOrEqual(String),
    Wildcard,                                // *
    Range(Box<Constraint>, Box<Constraint>), // >=1.0.0, <2.0.0
}

#[derive(Debug, Clone)]
pub struct ParsedVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub prerelease: Option<String>,
    pub build: Option<String>,
}

pub fn parse_version(version: &str) -> Result<ParsedVersion, String> {
    let version = version.trim_start_matches('v');

    // Split prerelease and build metadata
    let (version_part, prerelease, build) = if let Some(idx) = version.find('+') {
        let (v, b) = version.split_at(idx);
        let build = Some(b[1..].to_string());
        if let Some(pidx) = v.find('-') {
            let (base, pre) = v.split_at(pidx);
            (base, Some(pre[1..].to_string()), build)
        } else {
            (v, None, build)
        }
    } else if let Some(idx) = version.find('-') {
        let (v, p) = version.split_at(idx);
        (v, Some(p[1..].to_string()), None)
    } else {
        (version, None, None)
    };

    let parts: Vec<&str> = version_part.split('.').collect();

    if parts.is_empty() || parts.len() > 3 {
        return Err(format!("Invalid version format: {}", version));
    }

    let major = parts[0]
        .parse::<u32>()
        .map_err(|_| format!("Invalid major version: {}", parts[0]))?;

    let minor = parts
        .get(1)
        .unwrap_or(&"0")
        .parse::<u32>()
        .map_err(|_| format!("Invalid minor version"))?;

    let patch = parts
        .get(2)
        .unwrap_or(&"0")
        .parse::<u32>()
        .map_err(|_| format!("Invalid patch version"))?;

    Ok(ParsedVersion {
        major,
        minor,
        patch,
        prerelease,
        build,
    })
}

pub fn compare_versions(v1: &str, v2: &str) -> Ordering {
    let pv1 = match parse_version(v1) {
        Ok(v) => v,
        Err(_) => return Ordering::Equal,
    };
    let pv2 = match parse_version(v2) {
        Ok(v) => v,
        Err(_) => return Ordering::Equal,
    };

    // Compare major, minor, patch
    match pv1.major.cmp(&pv2.major) {
        Ordering::Equal => {}
        other => return other,
    }
    match pv1.minor.cmp(&pv2.minor) {
        Ordering::Equal => {}
        other => return other,
    }
    match pv1.patch.cmp(&pv2.patch) {
        Ordering::Equal => {}
        other => return other,
    }

    // Prerelease versions have lower precedence
    match (&pv1.prerelease, &pv2.prerelease) {
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (None, None) => Ordering::Equal,
        (Some(p1), Some(p2)) => compare_prerelease(p1, p2),
    }
}

fn compare_prerelease(p1: &str, p2: &str) -> Ordering {
    let parts1: Vec<&str> = p1.split('.').collect();
    let parts2: Vec<&str> = p2.split('.').collect();

    let max_len = parts1.len().max(parts2.len());

    for i in 0..max_len {
        let part1 = parts1.get(i);
        let part2 = parts2.get(i);

        match (part1, part2) {
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(a), Some(b)) => {
                // Numeric identifiers have lower precedence than alphanumeric
                let a_is_num = a.parse::<u32>().is_ok();
                let b_is_num = b.parse::<u32>().is_ok();

                match (a_is_num, b_is_num) {
                    (true, false) => return Ordering::Less,
                    (false, true) => return Ordering::Greater,
                    (true, true) => {
                        let na = a.parse::<u32>().unwrap();
                        let nb = b.parse::<u32>().unwrap();
                        match na.cmp(&nb) {
                            Ordering::Equal => continue,
                            other => return other,
                        }
                    }
                    (false, false) => match a.cmp(b) {
                        Ordering::Equal => continue,
                        other => return other,
                    },
                }
            }
        }
    }

    Ordering::Equal
}

pub fn parse_constraint(constraint: &str) -> Result<Constraint, String> {
    let constraint = constraint.trim();

    if constraint == "*" || constraint.is_empty() {
        return Ok(Constraint::Wildcard);
    }

    // Handle ranges (e.g., ">=1.0.0, <2.0.0")
    if constraint.contains(',') {
        let parts: Vec<&str> = constraint.split(',').map(|s| s.trim()).collect();
        if parts.len() == 2 {
            let lower = parse_constraint(parts[0])?;
            let upper = parse_constraint(parts[1])?;
            return Ok(Constraint::Range(Box::new(lower), Box::new(upper)));
        }
    }

    // Handle caret (^)
    if let Some(ver) = constraint.strip_prefix('^') {
        return Ok(Constraint::Caret(ver.to_string()));
    }

    // Handle tilde (~)
    if let Some(ver) = constraint.strip_prefix('~') {
        return Ok(Constraint::Tilde(ver.to_string()));
    }

    // Handle >=
    if let Some(ver) = constraint.strip_prefix(">=") {
        return Ok(Constraint::GreaterThanOrEqual(ver.trim().to_string()));
    }

    // Handle >
    if let Some(ver) = constraint.strip_prefix('>') {
        return Ok(Constraint::GreaterThan(ver.trim().to_string()));
    }

    // Handle <=
    if let Some(ver) = constraint.strip_prefix("<=") {
        return Ok(Constraint::LessThanOrEqual(ver.trim().to_string()));
    }

    // Handle <
    if let Some(ver) = constraint.strip_prefix('<') {
        return Ok(Constraint::LessThan(ver.trim().to_string()));
    }

    // Handle = (exact)
    if let Some(ver) = constraint.strip_prefix('=') {
        return Ok(Constraint::Exact(ver.trim().to_string()));
    }

    // Default to exact version
    Ok(Constraint::Exact(constraint.to_string()))
}

pub fn satisfies(version: &str, constraint: &str) -> bool {
    let constraint = match parse_constraint(constraint) {
        Ok(c) => c,
        Err(_) => return false,
    };

    satisfies_constraint(version, &constraint)
}

fn satisfies_constraint(version: &str, constraint: &Constraint) -> bool {
    match constraint {
        Constraint::Wildcard => true,
        Constraint::Exact(v) => compare_versions(version, v) == Ordering::Equal,
        Constraint::GreaterThan(v) => compare_versions(version, v) == Ordering::Greater,
        Constraint::GreaterThanOrEqual(v) => compare_versions(version, v) != Ordering::Less,
        Constraint::LessThan(v) => compare_versions(version, v) == Ordering::Less,
        Constraint::LessThanOrEqual(v) => compare_versions(version, v) != Ordering::Greater,
        Constraint::Caret(v) => satisfies_caret(version, v),
        Constraint::Tilde(v) => satisfies_tilde(version, v),
        Constraint::Range(lower, upper) => {
            satisfies_constraint(version, lower) && satisfies_constraint(version, upper)
        }
    }
}

fn satisfies_caret(version: &str, constraint: &str) -> bool {
    let pv = match parse_version(version) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let pc = match parse_version(constraint) {
        Ok(v) => v,
        Err(_) => return false,
    };

    // ^0.0.x is only exact match
    if pc.major == 0 && pc.minor == 0 {
        return pv.major == 0 && pv.minor == 0 && pv.patch == pc.patch;
    }

    // ^0.x is compatible with 0.x.y where y >= constraint patch
    if pc.major == 0 {
        return pv.major == 0 && pv.minor == pc.minor && pv.patch >= pc.patch;
    }

    // ^x.y.z is compatible with >=x.y.z <(x+1).0.0
    pv.major == pc.major && (pv.minor > pc.minor || (pv.minor == pc.minor && pv.patch >= pc.patch))
}

fn satisfies_tilde(version: &str, constraint: &str) -> bool {
    let pv = match parse_version(version) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let pc = match parse_version(constraint) {
        Ok(v) => v,
        Err(_) => return false,
    };

    // ~1.2.3 := >=1.2.3, <1.3.0
    // ~1.2 := >=1.2.0, <1.3.0
    // ~1 := >=1.0.0, <2.0.0

    if pc.major != pv.major {
        return false;
    }

    if pc.minor != pv.minor {
        return false;
    }

    pv.patch >= pc.patch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        let v = parse_version("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);

        let v = parse_version("2.0.0-alpha.1+build.123").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);
        assert_eq!(v.prerelease, Some("alpha.1".to_string()));
        assert_eq!(v.build, Some("build.123".to_string()));
    }

    #[test]
    fn test_compare_versions() {
        assert_eq!(compare_versions("1.0.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("1.0.0", "1.0.1"), Ordering::Less);
        assert_eq!(compare_versions("1.1.0", "1.0.0"), Ordering::Greater);
        assert_eq!(compare_versions("2.0.0", "1.9.9"), Ordering::Greater);
    }

    #[test]
    fn test_caret_constraint() {
        assert!(satisfies("1.2.3", "^1.2.3"));
        assert!(satisfies("1.2.4", "^1.2.3"));
        assert!(satisfies("1.3.0", "^1.2.3"));
        assert!(!satisfies("2.0.0", "^1.2.3"));
        assert!(!satisfies("1.2.2", "^1.2.3"));
    }

    #[test]
    fn test_tilde_constraint() {
        assert!(satisfies("1.2.3", "~1.2.3"));
        assert!(satisfies("1.2.4", "~1.2.3"));
        assert!(!satisfies("1.3.0", "~1.2.3"));
        assert!(!satisfies("1.2.2", "~1.2.3"));
    }

    #[test]
    fn test_exact_constraint() {
        assert!(satisfies("1.0.0", "1.0.0"));
        assert!(!satisfies("1.0.1", "1.0.0"));
    }

    #[test]
    fn test_greater_than_constraint() {
        assert!(satisfies("2.0.0", ">=1.0.0"));
        assert!(!satisfies("0.9.0", ">=1.0.0"));
    }
}
