use chrono::{Duration, Utc};
use uuid::Uuid;

use auth_api::services::account_security::{
    analyze_ip_reputation, base32_decode, base32_encode, compute_device_fingerprint,
    compute_totp_code, detect_impossible_travel, generate_backup_codes, generate_passkey_options,
    generate_totp_secret, hash_backup_code, haversine_distance_km, parse_device_details,
    verify_totp_code,
};

#[test]
fn test_base32_encoding_and_decoding_roundtrip() {
    let inputs: Vec<&[u8]> = vec![
        b"",
        b"f",
        b"fo",
        b"foo",
        b"foob",
        b"fooba",
        b"foobar",
        b"AuthenticationSecurity2026",
    ];

    for input in inputs {
        let encoded = base32_encode(input);
        let decoded = base32_decode(&encoded).expect("decoding must succeed");
        assert_eq!(decoded, input);
    }
}

#[test]
fn test_rfc6238_totp_generation_and_drift_verification() {
    let secret = generate_totp_secret();
    assert!(!secret.is_empty());

    let secret_bytes = base32_decode(&secret).expect("secret must be valid base32");
    assert_eq!(secret_bytes.len(), 20);

    let now_secs = Utc::now().timestamp() as u64;
    let code = compute_totp_code(&secret_bytes, now_secs);

    assert_eq!(code.len(), 6);
    assert!(code.chars().all(|c| c.is_ascii_digit()));

    assert!(verify_totp_code(&secret, &code, 1));

    assert!(!verify_totp_code(&secret, "000000", 1) || code == "000000");
    assert!(!verify_totp_code(&secret, "12345", 1));
    assert!(!verify_totp_code(&secret, "abcdef", 1));
}

#[test]
fn test_backup_codes_generation_and_hashing() {
    let codes = generate_backup_codes(10);
    assert_eq!(codes.len(), 10);

    for code in &codes {
        assert_eq!(code.len(), 9);
        assert_eq!(&code[4..5], "-");

        let hash1 = hash_backup_code(code);
        let hash2 = hash_backup_code(&code.to_lowercase());
        let hash3 = hash_backup_code(&code.replace('-', ""));

        assert_eq!(hash1.len(), 64);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1, hash3);
    }
}

#[test]
fn test_passkey_options_generation() {
    let user_id = Uuid::new_v4();
    let email = "security-officer@signet.test";
    let display_name = "Security Officer";

    let options = generate_passkey_options(user_id, email, display_name);

    assert_eq!(options.user_id, user_id.to_string());
    assert_eq!(options.user_email, email);
    assert_eq!(options.user_display_name, display_name);
    assert!(!options.challenge.is_empty());
    assert_eq!(options.rp_id, "localhost");
}

#[test]
fn test_impossible_travel_detection() {
    let ny_coords = (40.7128, -74.0060);
    let sf_coords = (37.7749, -122.4194);
    let tokyo_coords = (35.6762, 139.6503);

    let ny_to_sf_km = haversine_distance_km(ny_coords.0, ny_coords.1, sf_coords.0, sf_coords.1);
    assert!(ny_to_sf_km > 3500.0 && ny_to_sf_km < 4500.0);

    let t0 = Utc::now();

    let t_normal = t0 + Duration::hours(7);
    let normal_travel = detect_impossible_travel(Some(ny_coords), t0, Some(sf_coords), t_normal);
    assert!(normal_travel.is_none());

    let t_impossible = t0 + Duration::minutes(20);
    let impossible_travel =
        detect_impossible_travel(Some(ny_coords), t0, Some(tokyo_coords), t_impossible);
    assert!(impossible_travel.is_some());

    let (velocity, desc) = impossible_travel.unwrap();
    assert!(velocity > 800.0);
    assert!(desc.contains("exceeding aircraft velocity"));
}

#[test]
fn test_ip_reputation_analysis_matrix() {
    let loopback = analyze_ip_reputation("127.0.0.1");
    assert_eq!(loopback.score, 0);
    assert_eq!(loopback.risk_level, "safe");
    assert_eq!(loopback.classification, "local_loopback");

    let private_ip = analyze_ip_reputation("192.168.1.100");
    assert_eq!(private_ip.score, 5);
    assert_eq!(private_ip.risk_level, "safe");
    assert_eq!(private_ip.classification, "private_network");

    let datacenter_ip = analyze_ip_reputation("34.120.50.2");
    assert_eq!(datacenter_ip.score, 40);
    assert_eq!(datacenter_ip.risk_level, "low");
    assert!(datacenter_ip.is_datacenter);

    let vpn_ip = analyze_ip_reputation("185.220.101.5");
    assert_eq!(vpn_ip.score, 65);
    assert_eq!(vpn_ip.risk_level, "moderate");
    assert!(vpn_ip.is_vpn_proxy);

    let tor_ip = analyze_ip_reputation("198.51.100.99");
    assert_eq!(tor_ip.score, 90);
    assert_eq!(tor_ip.risk_level, "severe");
    assert!(tor_ip.is_tor);
}

#[test]
fn test_device_fingerprinting_and_parsing() {
    let mac_safari = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15";
    let win_chrome = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
    let iphone_safari = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";

    let (dev1, type1) = parse_device_details(Some(mac_safari));
    assert_eq!(dev1, "Safari on macOS");
    assert_eq!(type1, "desktop");

    let (dev2, type2) = parse_device_details(Some(win_chrome));
    assert_eq!(dev2, "Chrome on Windows");
    assert_eq!(type2, "desktop");

    let (dev3, type3) = parse_device_details(Some(iphone_safari));
    assert_eq!(dev3, "Safari on iOS");
    assert_eq!(type3, "mobile");

    let fp1 = compute_device_fingerprint(Some(mac_safari), Some("192.168.1.50"));
    let fp2 = compute_device_fingerprint(Some(mac_safari), Some("192.168.1.51"));
    let fp3 = compute_device_fingerprint(Some(win_chrome), Some("192.168.1.50"));

    assert_eq!(fp1, fp2);
    assert_ne!(fp1, fp3);
}
