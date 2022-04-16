pub fn power_of_ten(mut n: u64) -> Option<u64> {
    let mut pow = 1;
    let mut r = 0;
    loop {
        if r > 0 || n == 0 {
            return None;
        } else if n == 10 {
            return Some(pow);
        } else {
            pow += 1;
            r = n % 10;
            n /= 10;
        }
    }
}

pub fn format_display_name(name: &str) -> String {
    name.split('/')
        .map(|part| part.split_once('-').unwrap().1.replace("-", " "))
        .collect::<Vec<String>>()
        .join(" / ")
}

pub fn prepare_path_name(node_name: &str) -> String {
    node_name
        // .to_ascii_lowercase()
        .chars()
        .map(|c| match c {
            ' ' => '-',
            _ => c,
        })
        .collect()
}

#[test]
fn test_power_of_ten() {
    assert_eq!(power_of_ten(0), None);
    assert_eq!(power_of_ten(9), None);
    assert_eq!(power_of_ten(11), None);
    assert_eq!(power_of_ten(10), Some(1));
    assert_eq!(power_of_ten(100), Some(2));
    assert_eq!(power_of_ten(100_000), Some(5));
}

#[test]
fn test_format_display_name() {
    assert!(format_display_name("002-desk") == *"desk");
    assert!(format_display_name("002-desk/1-cool-jazz") == *"desk / cool jazz");
}
