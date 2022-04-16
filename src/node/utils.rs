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

pub fn link_text_is_node_name(link_text: &String, node_path: &String) -> bool {
    let text_len = link_text.len();
    let path_len = node_path.len();
    if path_len > text_len {
        for (a, b) in node_path[path_len - text_len..]
            .chars()
            .zip(link_text.chars().map(|c| match c {
                ' ' => '-',
                _ => c,
            }))
        {
            if a != b {
                return false;
            }
        }
        true
    } else {
        false
    }
}

#[test]
fn test_link_text_is_node_name() {
    assert!(link_text_is_node_name(
        &"Journal".to_string(),
        &"01-Journal".to_string()
    ));
    assert!(!link_text_is_node_name(
        &"journal".to_string(),
        &"01-Journal".to_string()
    ));
    assert!(link_text_is_node_name(
        &"jazz-node".to_string(),
        &"1-Desk/0001-jazz-node".to_string()
    ));
    assert!(link_text_is_node_name(
        &"A Show About Nothing".to_string(),
        &"1-Desk/0101-A-Show-About-Nothing".to_string()
    ));
    assert!(!link_text_is_node_name(
        &"A SHOW ABOUT NOTHING".to_string(),
        &"1-Desk/0101-a-show-about-nothing".to_string()
    ));
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
