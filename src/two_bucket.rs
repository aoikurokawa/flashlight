#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Bucket {
    One,
    Two,
}

/// A struct to hold your results in.
#[derive(PartialEq, Eq, Debug)]
pub struct BucketStats {
    /// The total number of "moves" it should take to reach the desired number of liters, including
    /// the first fill.
    pub moves: u8,
    /// Which bucket should end up with the desired number of liters? (Either "one" or "two")
    pub goal_bucket: Bucket,
    /// How many liters are left in the other bucket?
    pub other_bucket: u8,
}

#[derive(Copy, Clone)]
struct BucketStruct {
    capacity: u8,
    contained: u8,
    bucket: Bucket,
}

impl BucketStruct {
    fn new(capacity: u8, contained: u8, bucket: Bucket) -> Self {
        BucketStruct {
            capacity,
            contained,
            bucket,
        }
    }

    fn fill(&mut self) {
        self.contained = self.capacity;
    }

    fn empty(&mut self) {
        self.contained = 0;
    }

    fn is_full(&self) -> bool {
        self.capacity == self.contained
    }

    fn is_empty(&self) -> bool {
        self.contained == 0
    }

    fn is_goal(&self, goal: u8) -> bool {
        self.contained == goal
    }

    fn fill_with(&mut self, other: &mut BucketStruct) {
        let available = self.capacity - self.contained;
        self.contained += if other.contained >= available {
            other.contained -= available;
            available
        } else {
            let added = other.contained;
            other.contained = 0;
            added
        }
    }
}

/// Solve the bucket problem
pub fn solve(
    capacity_1: u8,
    capacity_2: u8,
    goal: u8,
    start_bucket: &Bucket,
) -> Option<BucketStats> {
    if (goal > 1 && capacity_1 % goal == 0 && goal != capacity_1)
        || (goal > 1 && capacity_2 % goal == 0 && goal != capacity_2)
    {
        return None;
    }

    let bucket_one = BucketStruct::new(capacity_1, 0, Bucket::One);
    let bucket_two = BucketStruct::new(capacity_2, 0, Bucket::Two);

    let (mut from, mut to) = match start_bucket {
        Bucket::One => (bucket_one, bucket_two),
        Bucket::Two => (bucket_two, bucket_one),
    };
    from.fill();
    let mut moves = 1;

    if goal == to.capacity && goal != from.capacity {
        to.fill();
        moves += 1;
    }

    loop {
        if [from, to].iter().any(|bucket| bucket.is_goal(goal)) {
            let (goal_bucket, other_bucket) = if from.is_goal(goal) {
                (from.bucket, to.contained)
            } else {
                (to.bucket, from.contained)
            };
            let stats = BucketStats {
                moves,
                goal_bucket,
                other_bucket,
            };
            return Some(stats);
        }
        if from.is_empty() {
            from.fill();
            moves += 1;
        }
        if to.is_full() {
            to.empty();
            moves += 1;
        }
        to.fill_with(&mut from);
        moves += 1;
    }
}

#[cfg(test)]
mod tests {
    use crate::two_bucket::{solve, Bucket, BucketStats};

    #[test]
    fn test_case_1() {
        assert_eq!(
            solve(3, 5, 1, &Bucket::One),
            Some(BucketStats {
                moves: 4,

                goal_bucket: Bucket::One,

                other_bucket: 5,
            })
        );
    }

    #[test]
    fn test_case_2() {
        assert_eq!(
            solve(3, 5, 1, &Bucket::Two),
            Some(BucketStats {
                moves: 8,

                goal_bucket: Bucket::Two,

                other_bucket: 3,
            })
        );
    }

    #[test]
    fn test_case_3() {
        assert_eq!(
            solve(7, 11, 2, &Bucket::One),
            Some(BucketStats {
                moves: 14,

                goal_bucket: Bucket::One,

                other_bucket: 11,
            })
        );
    }

    #[test]
    fn test_case_4() {
        assert_eq!(
            solve(7, 11, 2, &Bucket::Two),
            Some(BucketStats {
                moves: 18,

                goal_bucket: Bucket::Two,

                other_bucket: 7,
            })
        );
    }

    #[test]
    fn goal_equal_to_start_bucket() {
        assert_eq!(
            solve(1, 3, 3, &Bucket::Two),
            Some(BucketStats {
                moves: 1,

                goal_bucket: Bucket::Two,

                other_bucket: 0,
            })
        );
    }

    #[test]
    fn goal_equal_to_other_bucket() {
        assert_eq!(
            solve(2, 3, 3, &Bucket::One),
            Some(BucketStats {
                moves: 2,

                goal_bucket: Bucket::Two,

                other_bucket: 2,
            })
        );
    }

    #[test]
    fn not_possible_to_reach_the_goal() {
        assert_eq!(solve(6, 15, 5, &Bucket::One), None);
    }

    #[test]
    fn with_same_buckets_but_different_goal_then_it_is_possible() {
        assert_eq!(
            solve(6, 15, 9, &Bucket::One),
            Some(BucketStats {
                moves: 10,

                goal_bucket: Bucket::Two,

                other_bucket: 0,
            })
        );
    }
}
