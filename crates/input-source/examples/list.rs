//! Prints `id<TAB>name` for every selectable keyboard input source.
//! Use it to find the IDs to put in Downlink's config.toml.
fn main() {
    match input_source::system_backend().list() {
        Ok(sources) => {
            for source in sources {
                println!("{}\t{}", source.id, source.name);
            }
        }
        Err(error) => eprintln!("could not list input sources: {error}"),
    }
}
