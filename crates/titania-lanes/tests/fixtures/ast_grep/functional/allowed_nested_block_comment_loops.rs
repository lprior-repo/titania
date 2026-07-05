// Fixture: allowed — nested block comments with loop keywords.

/*
outer comment starts
/* nested comment starts
for item in items { do_work(item) }
*/
while still_inside_outer { retry() }
loop { still_inside_outer(); }
*/
pub fn process_all(items: &[i32]) -> Vec<i32> {
    items.iter().map(|item| item * 2).collect()
}
