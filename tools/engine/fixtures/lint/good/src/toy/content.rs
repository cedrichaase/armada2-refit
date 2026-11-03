// lint fixture: the toy plugin content sets
pub fn by_set(id: &str) -> u32 {
    match id {
        "stock" => 1,
        _ => 0,
    }
}
