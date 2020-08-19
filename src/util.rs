pub(crate) fn new_capacity_amortized(
    current_capacity: usize,
    required_capacity: usize,
) -> Option<usize> {
    if current_capacity < required_capacity {
        let mut new_capacity = current_capacity;

        if new_capacity == 0 {
            new_capacity = 2;
        } else {
            while new_capacity < required_capacity {
                new_capacity = new_capacity * 2;
            }
        }

        Some(new_capacity)
    } else {
        None
    }
}
