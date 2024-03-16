use clap::Subcommand;


#[derive(Subcommand)]
pub enum ModifyCommands {
    Add,
    Remove,
    Update,
}

pub fn run_modify(
    _parsed_schema: &arc_isle::schema::Schema,
    _command: ModifyCommands,
) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
