use ::std::collections::HashMap;


pub fn parse_data(input: &str) -> HashMap<String, String> {
    input
        .split(',')
        .map(|pair| {
            let parts: Vec<&str> = pair.split(':').collect();
            if parts.len() == 2 {
                (String::from(parts[0]), String::from(parts[1]))
            } else {
                // Handle invalid format if needed
                // For example, you can return a default or placeholder value
                (String::new(), String::new())
            }
        })
        .collect()
}


pub fn print_usage() {
    println!("Invalid arguments")
}