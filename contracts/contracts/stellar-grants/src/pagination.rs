use soroban_sdk::{Env, IntoVal, TryFromVal, Val, Vec};

use crate::constants::MAX_PAGE_SIZE;

/// Slice `items[offset..offset+limit]` (clamped to `MAX_PAGE_SIZE` and the
/// length of `items`) into a new `Vec`. Used by any module that needs
/// offset/limit pagination over a `soroban_sdk::Vec` (registry, audit log,
/// grants, etc.) so the slicing logic lives in exactly one place.
pub fn paginate<T>(env: &Env, items: &Vec<T>, offset: u32, limit: u32) -> Vec<T>
where
    T: IntoVal<Env, Val> + TryFromVal<Env, Val>,
{
    let total = items.len();
    let effective_limit = if limit > MAX_PAGE_SIZE {
        MAX_PAGE_SIZE
    } else {
        limit
    };

    let mut result: Vec<T> = Vec::new(env);
    if offset >= total || effective_limit == 0 {
        return result;
    }

    let mut i = offset;
    while i < total && i - offset < effective_limit {
        if let Some(item) = items.get(i) {
            result.push_back(item);
        }
        i += 1;
    }

    result
}

/// Total number of pages needed to cover `total_len` items at `page_size` per page.
/// Page size is clamped to at least 1 to avoid division by zero.
pub fn page_count(total_len: u32, page_size: u32) -> u32 {
    let size = if page_size == 0 { 1 } else { page_size };
    total_len.div_ceil(size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    fn make_vec(env: &Env, n: u32) -> Vec<u32> {
        let mut v = Vec::new(env);
        for i in 0..n {
            v.push_back(i);
        }
        v
    }

    #[test]
    fn test_paginate_basic_slice() {
        let env = Env::default();
        let items = make_vec(&env, 10);
        let page = paginate(&env, &items, 2, 3);
        assert_eq!(page.len(), 3);
        assert_eq!(page.get(0), Some(2));
        assert_eq!(page.get(2), Some(4));
    }

    #[test]
    fn test_paginate_offset_past_end_returns_empty() {
        let env = Env::default();
        let items = make_vec(&env, 5);
        let page = paginate(&env, &items, 10, 3);
        assert_eq!(page.len(), 0);
    }

    #[test]
    fn test_paginate_limit_clamped_to_max_page_size() {
        let env = Env::default();
        let items = make_vec(&env, MAX_PAGE_SIZE + 20);
        let page = paginate(&env, &items, 0, MAX_PAGE_SIZE + 20);
        assert_eq!(page.len(), MAX_PAGE_SIZE);
    }

    #[test]
    fn test_paginate_zero_limit_returns_empty() {
        let env = Env::default();
        let items = make_vec(&env, 5);
        let page = paginate(&env, &items, 0, 0);
        assert_eq!(page.len(), 0);
    }

    #[test]
    fn test_paginate_last_partial_page() {
        let env = Env::default();
        let items = make_vec(&env, 7);
        let page = paginate(&env, &items, 5, 10);
        assert_eq!(page.len(), 2);
        assert_eq!(page.get(0), Some(5));
        assert_eq!(page.get(1), Some(6));
    }

    #[test]
    fn test_paginate_empty_input() {
        let env = Env::default();
        let items: Vec<u32> = Vec::new(&env);
        let page = paginate(&env, &items, 0, 10);
        assert_eq!(page.len(), 0);
    }

    #[test]
    fn test_page_count_exact_division() {
        assert_eq!(page_count(20, 10), 2);
    }

    #[test]
    fn test_page_count_remainder_rounds_up() {
        assert_eq!(page_count(21, 10), 3);
    }

    #[test]
    fn test_page_count_zero_total() {
        assert_eq!(page_count(0, 10), 0);
    }

    #[test]
    fn test_page_count_zero_page_size_treated_as_one() {
        assert_eq!(page_count(5, 0), 5);
    }
}
