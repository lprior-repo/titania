// Fixture: allowed — for-loop text only in whole-line comments.
//
// The detectors skip lines starting with `//` after trimming, so this
// file must stay clean under FUNC_LOOPS_FOR.

// for x in items { do_work(x) }
// for item in collection { process(item) }
//      for key in map { handle_key(key) }

pub fn process_items(items: &[i32]) -> i32 {
    items.iter().sum()
}
