use std::{collections::HashSet, hash::Hash};

#[derive(Debug, PartialEq, Eq)]
pub struct CustomSet<T> {
    value: Vec<T>,
}

impl<T> CustomSet<T>
where
    T: Eq + PartialEq + Clone + Copy + Hash + Ord,
{
    #[allow(clippy::all)]
    pub fn new(input: &[T]) -> Self {
        let mut value = input.clone().to_vec();
        value.sort();
        Self {
            value,
        }
    }

    pub fn contains(&self, element: &T) -> bool {
        self.value.contains(element)
    }

    pub fn add(&mut self, element: T) {
        if !self.contains(&element) {
            self.value.push(element);
            self.value.sort();
        }
    }

    pub fn is_subset(&self, other: &Self) -> bool {
        match (self.is_empty(), other.is_empty()) {
            (true, true) => return true,
            (true, false) => return true,
            (false, true) => return false,
            _ => {},
        }

        let mut valid = false;
        other.value.windows(self.value.len()).for_each(|candidate| {
            if candidate == self.value {
                valid = true;
            }
        });
        valid
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    pub fn is_disjoint(&self, other: &Self) -> bool {
        for val in self.value.iter() {
            for other_val in other.value.iter() {
                if val == other_val {
                    return false;
                }
            }
        }

        true
    }

    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        let mut candidates = Vec::new();

        for val in self.value.iter() {
            for other_val in other.value.iter() {
                if val == other_val {
                    candidates.push(*val);
                }
            }
        }

        candidates.sort();

        CustomSet::new(candidates.as_slice())
    }

    #[must_use]
    pub fn difference(&self, other: &Self) -> Self {
        let mut set = HashSet::new();

        self.value.iter().for_each(|val| {
            set.insert(*val);
        });

        for val in self.value.iter() {
            for other_val in other.value.iter() {
                if val == other_val {
                    set.remove(val);
                }
            }
        }

        let mut candidate = set.into_iter().collect::<Vec<_>>();
        candidate.sort();
        CustomSet::new(&candidate)
    }

    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        let mut set = HashSet::new();

        self.value.iter().for_each(|val| {
            set.insert(*val);
        });

        other.value.iter().for_each(|val| {
            set.insert(*val);
        });

        let candidate = set.into_iter().collect::<Vec<_>>();
        CustomSet::new(&candidate)
    }
}

#[cfg(test)]
mod tests {
    use crate::custom_set::*;

    #[test]
    fn sets_with_no_elements_are_empty() {
        let set: CustomSet<()> = CustomSet::new(&[]);

        assert!(set.is_empty());
    }

    #[test]
    fn sets_with_elements_are_not_empty() {
        let set = CustomSet::new(&[1]);

        assert!(!set.is_empty());
    }

    #[test]
    fn nothing_is_contained_in_an_empty_set() {
        let set = CustomSet::new(&[]);

        assert!(!set.contains(&1));
    }

    #[test]
    fn true_when_the_element_is_in_the_set() {
        let set = CustomSet::new(&[1, 2, 3]);

        assert!(set.contains(&1));
    }

    #[test]
    fn false_when_the_element_is_not_in_the_set() {
        let set = CustomSet::new(&[1, 2, 3]);

        assert!(!set.contains(&4));
    }

    #[test]
    fn empty_sets_are_subsets_of_each_other() {
        let set1: CustomSet<()> = CustomSet::new(&[]);

        let set2: CustomSet<()> = CustomSet::new(&[]);

        assert!(set1.is_subset(&set2));

        assert!(set2.is_subset(&set1));
    }

    #[test]
    fn empty_set_is_subset_of_non_empty_set() {
        let set1 = CustomSet::new(&[]);

        let set2 = CustomSet::new(&[1]);

        assert!(set1.is_subset(&set2));
    }

    #[test]
    fn non_empty_set_is_not_subset_of_empty_set() {
        let set1 = CustomSet::new(&[1]);

        let set2 = CustomSet::new(&[]);

        assert!(!set1.is_subset(&set2));
    }

    #[test]
    fn sets_with_same_elements_are_subsets() {
        let set1 = CustomSet::new(&[1, 2, 3]);

        let set2 = CustomSet::new(&[1, 2, 3]);

        assert!(set1.is_subset(&set2));

        assert!(set2.is_subset(&set1));
    }

    #[test]
    fn set_contained_in_other_set_is_a_subset() {
        let set1 = CustomSet::new(&[1, 2, 3]);

        let set2 = CustomSet::new(&[4, 1, 2, 3]);

        assert!(set1.is_subset(&set2));
    }

    #[test]
    fn set_not_contained_in_other_set_is_not_a_subset_one() {
        let set1 = CustomSet::new(&[1, 2, 3]);

        let set2 = CustomSet::new(&[4, 1, 3]);

        assert!(!set1.is_subset(&set2));
    }

    #[test]
    fn empty_sets_are_disjoint_with_each_other() {
        let set1: CustomSet<()> = CustomSet::new(&[]);

        let set2: CustomSet<()> = CustomSet::new(&[]);

        assert!(set1.is_disjoint(&set2));

        assert!(set2.is_disjoint(&set1));
    }

    #[test]
    fn empty_set_disjoint_with_non_empty_set() {
        let set1 = CustomSet::new(&[]);

        let set2 = CustomSet::new(&[1]);

        assert!(set1.is_disjoint(&set2));
    }

    #[test]
    fn non_empty_set_disjoint_with_empty_set() {
        let set1 = CustomSet::new(&[1]);

        let set2 = CustomSet::new(&[]);

        assert!(set1.is_disjoint(&set2));
    }

    #[test]
    fn sets_with_one_element_in_common_are_not_disjoint() {
        let set1 = CustomSet::new(&[1, 2]);

        let set2 = CustomSet::new(&[2, 3]);

        assert!(!set1.is_disjoint(&set2));

        assert!(!set2.is_disjoint(&set1));
    }

    #[test]
    fn sets_with_no_elements_in_common_are_disjoint() {
        let set1 = CustomSet::new(&[1, 2]);

        let set2 = CustomSet::new(&[3, 4]);

        assert!(set1.is_disjoint(&set2));

        assert!(set2.is_disjoint(&set1));
    }

    #[test]
    fn empty_sets_are_equal() {
        let set1: CustomSet<()> = CustomSet::new(&[]);

        let set2: CustomSet<()> = CustomSet::new(&[]);

        assert_eq!(set1, set2);
    }

    #[test]
    fn empty_set_is_not_equal_to_a_non_empty_set() {
        let set1 = CustomSet::new(&[]);

        let set2 = CustomSet::new(&[1, 2, 3]);

        assert_ne!(set1, set2);
    }

    #[test]
    fn non_empty_set_is_not_equal_to_an_empty_set() {
        let set1 = CustomSet::new(&[1, 2, 3]);

        let set2 = CustomSet::new(&[]);

        assert_ne!(set1, set2);
    }

    #[test]
    fn sets_with_the_same_elements_are_equal() {
        let set1 = CustomSet::new(&[1, 2]);

        let set2 = CustomSet::new(&[2, 1]);

        assert_eq!(set1, set2);
    }

    #[test]
    fn sets_with_different_elements_are_not_equal() {
        let set1 = CustomSet::new(&[1, 2, 3]);

        let set2 = CustomSet::new(&[2, 1, 4]);

        assert_ne!(set1, set2);
    }

    #[test]
    fn add_to_empty_set() {
        let mut set = CustomSet::new(&[]);

        set.add(3);

        assert_eq!(set, CustomSet::new(&[3]));
    }

    #[test]
    fn add_to_non_empty_set() {
        let mut set = CustomSet::new(&[1, 2, 4]);

        set.add(3);

        assert_eq!(set, CustomSet::new(&[1, 2, 3, 4]));
    }

    #[test]
    fn add_existing_element() {
        let mut set = CustomSet::new(&[1, 2, 3]);

        set.add(3);

        assert_eq!(set, CustomSet::new(&[1, 2, 3]));
    }

    #[test]
    fn intersecting_empty_sets_return_empty_set() {
        let set1: CustomSet<()> = CustomSet::new(&[]);

        let set2: CustomSet<()> = CustomSet::new(&[]);

        assert_eq!(set1.intersection(&set2), CustomSet::new(&[]));
    }

    #[test]
    fn intersecting_empty_set_with_non_empty_returns_empty_set() {
        let set1 = CustomSet::new(&[]);

        let set2 = CustomSet::new(&[3, 2, 5]);

        assert_eq!(set1.intersection(&set2), CustomSet::new(&[]));
    }

    #[test]
    fn intersecting_non_empty_set_with_empty_returns_empty_set() {
        let set1 = CustomSet::new(&[1, 2, 3, 4]);

        let set2 = CustomSet::new(&[]);

        assert_eq!(set1.intersection(&set2), CustomSet::new(&[]));
    }

    #[test]
    fn intersection_of_two_sets_with_no_shared_elements_is_an_empty_set() {
        let set1 = CustomSet::new(&[1, 2, 3]);

        let set2 = CustomSet::new(&[4, 5, 6]);

        assert_eq!(set1.intersection(&set2), CustomSet::new(&[]));

        assert_eq!(set2.intersection(&set1), CustomSet::new(&[]));
    }

    #[test]
    fn intersection_of_two_sets_with_shared_elements_is_a_set_of_the_shared_elements() {
        let set1 = CustomSet::new(&[1, 2, 3, 4]);

        let set2 = CustomSet::new(&[3, 2, 5]);

        assert_eq!(set1.intersection(&set2), CustomSet::new(&[2, 3]));

        assert_eq!(set2.intersection(&set1), CustomSet::new(&[2, 3]));
    }

    #[test]
    fn difference_of_two_empty_sets_is_empty_set() {
        let set1: CustomSet<()> = CustomSet::new(&[]);

        let set2: CustomSet<()> = CustomSet::new(&[]);

        assert_eq!(set1.difference(&set2), CustomSet::new(&[]));
    }

    #[test]
    fn difference_of_an_empty_and_non_empty_set_is_an_empty_set() {
        let set1 = CustomSet::new(&[]);

        let set2 = CustomSet::new(&[3, 2, 5]);

        assert_eq!(set1.difference(&set2), CustomSet::new(&[]));
    }

    #[test]
    fn difference_of_a_non_empty_set_and_empty_set_is_the_non_empty_set() {
        let set1 = CustomSet::new(&[1, 2, 3, 4]);

        let set2 = CustomSet::new(&[]);

        assert_eq!(set1.difference(&set2), CustomSet::new(&[1, 2, 3, 4]));
    }

    #[test]
    fn difference_of_two_non_empty_sets_is_elements_only_in_first_set_one() {
        let set1 = CustomSet::new(&[3, 2, 1]);

        let set2 = CustomSet::new(&[2, 4]);

        assert_eq!(set1.difference(&set2), CustomSet::new(&[1, 3]));
    }

    #[test]
    fn union_of_two_empty_sets_is_empty_set() {
        let set1: CustomSet<()> = CustomSet::new(&[]);

        let set2: CustomSet<()> = CustomSet::new(&[]);

        assert_eq!(set1.union(&set2), CustomSet::new(&[]));
    }

    #[test]
    fn union_of_empty_set_and_non_empty_set_is_all_elements() {
        let set1 = CustomSet::new(&[]);

        let set2 = CustomSet::new(&[2]);

        assert_eq!(set1.union(&set2), CustomSet::new(&[2]));
    }

    #[test]
    fn union_of_non_empty_set_and_empty_set_is_the_non_empty_set() {
        let set1 = CustomSet::new(&[1, 3]);

        let set2 = CustomSet::new(&[]);

        assert_eq!(set1.union(&set2), CustomSet::new(&[1, 3]));
    }

    #[test]
    fn union_of_non_empty_sets_contains_all_unique_elements() {
        let set1 = CustomSet::new(&[1, 3]);

        let set2 = CustomSet::new(&[2, 3]);

        assert_eq!(set1.union(&set2), CustomSet::new(&[3, 2, 1]));
    }
}
