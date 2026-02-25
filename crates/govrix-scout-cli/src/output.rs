use anyhow::Result;
use serde_json::Value;

pub fn print_output(format: &str, value: &Value) -> Result<()> {
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(value)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(value)?);
        }
        _ => {
            print_table(value);
        }
    }
    Ok(())
}

fn print_table(value: &Value) {
    match value {
        Value::Object(map) => {
            if let Some(Value::Array(rows)) = map.get("data") {
                print_rows(rows);
                for (k, v) in map {
                    if k != "data" {
                        println!("{}: {}", k, v);
                    }
                }
            } else if let Some(data) = map.get("data") {
                print_object(data);
            } else {
                print_object(value);
            }
        }
        Value::Array(rows) => print_rows(rows),
        _ => println!("{value}"),
    }
}

fn print_rows(rows: &[Value]) {
    if rows.is_empty() {
        println!("(no results)");
        return;
    }
    let cols: Vec<String> = if let Value::Object(map) = &rows[0] {
        map.keys().cloned().collect()
    } else {
        for r in rows {
            println!("{r}");
        }
        return;
    };

    let mut widths: Vec<usize> = cols.iter().map(|c| c.len()).collect();
    let cell_values: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            cols.iter()
                .enumerate()
                .map(|(i, col)| {
                    let s = cell_str(row.get(col));
                    widths[i] = widths[i].max(s.len().min(40));
                    s
                })
                .collect()
        })
        .collect();

    let header: Vec<String> = cols
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{:width$}", c, width = widths[i]))
        .collect();
    println!("{}", header.join("  "));
    let sep: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
    println!("{}", sep.join("  "));

    for cells in &cell_values {
        let line: Vec<String> = cells
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{:width$}", truncate(c, 40), width = widths[i]))
            .collect();
        println!("{}", line.join("  "));
    }
}

fn print_object(value: &Value) {
    if let Value::Object(map) = value {
        let max_key = map.keys().map(|k| k.len()).max().unwrap_or(0);
        for (k, v) in map {
            println!("{:>width$}: {}", k, cell_str(Some(v)), width = max_key);
        }
    } else {
        println!("{value}");
    }
}

fn cell_str(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => "-".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(other) => other.to_string(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max - 3])
    } else {
        s.to_string()
    }
}
