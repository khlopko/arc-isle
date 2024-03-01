// main.rs

mod parser;
mod schema;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let args: Vec<String> = env::args().collect();
    // dbg!(args);
    let file_content = parser::parse("example")?;
    println!("{}", file_content);
    Ok(())
}
