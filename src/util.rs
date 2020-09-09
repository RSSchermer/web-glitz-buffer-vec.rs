pub(crate) fn new_capacity_amortized(
    current_capacity: usize,
    required_capacity: usize,
) -> Option<usize> {
    if current_capacity < required_capacity {
        let mut new_capacity = current_capacity;

        if new_capacity == 0 {
            new_capacity = 2;
        }

        while new_capacity < required_capacity {
            new_capacity = new_capacity * 2;
        }

        Some(new_capacity)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::util::new_capacity_amortized;

    #[test]
    fn test_new_capacity_amortized() {
        assert_eq!(new_capacity_amortized(0, 0), None);
        assert_eq!(new_capacity_amortized(0, 1), Some(2));
        assert_eq!(new_capacity_amortized(2, 2), None);
        assert_eq!(new_capacity_amortized(2, 3), Some(4));
        assert_eq!(new_capacity_amortized(4, 4), None);
        assert_eq!(new_capacity_amortized(4, 5), Some(8));
    }
}
