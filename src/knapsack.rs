pub struct Item {
    pub weight: u32,
    pub value: u32,
}

pub fn maximum_value(max_weight: u32, items: &[Item]) -> u32 {
    let n = items.len();
    let mut dp = vec![vec![0; max_weight as usize + 1]; n + 1];

    for i in 1..=n {
        for w in 1..=max_weight as usize {
            if items[i - 1].weight as usize <= w {
                dp[i][w] = dp[i - 1][w]
                    .max(dp[i - 1][w - items[i - 1].weight as usize] + items[i - 1].value);
            } else {
                dp[i][w] = dp[i - 1][w];
            }
        }
    }

    println!("{:?}", dp);

    dp[n][max_weight as usize]
}

#[cfg(test)]
mod tests {
    use crate::knapsack::*;

    #[test]
    fn test_example_knapsack() {
        let max_weight = 10;

        let items = [
            Item {
                weight: 5,

                value: 10,
            },
            Item {
                weight: 4,

                value: 40,
            },
            Item {
                weight: 6,

                value: 30,
            },
            Item {
                weight: 4,

                value: 50,
            },
        ];

        // [[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], [0, 0, 0, 0, 0, 10, 10, 10, 10, 10, 10], [0, 0, 0, 0, 40, 40, 40, 40, 40, 50, 50], [0, 0, 0, 0, 40, 40, 40, 40, 40, 50, 70], [0, 0, 0, 0, 50, 50, 50, 50, 90, 90, 90]]

        assert_eq!(maximum_value(max_weight, &items), 90);
    }

    #[test]
    fn test_no_items() {
        let max_weight = 100;

        let items = [];

        assert_eq!(maximum_value(max_weight, &items), 0);
    }

    #[test]
    fn test_one_item_too_heavy() {
        let max_weight = 10;

        let items = [Item {
            weight: 100,

            value: 1,
        }];

        assert_eq!(maximum_value(max_weight, &items), 0);
    }

    #[test]
    fn test_five_items_cannot_be_greedy_by_weight() {
        let max_weight = 10;

        let items = [
            Item {
                weight: 2,

                value: 5,
            },
            Item {
                weight: 2,

                value: 5,
            },
            Item {
                weight: 2,

                value: 5,
            },
            Item {
                weight: 2,

                value: 5,
            },
            Item {
                weight: 10,

                value: 21,
            },
        ];

        assert_eq!(maximum_value(max_weight, &items), 21);
    }

    #[test]
    fn test_five_items_cannot_be_greedy_by_value() {
        let max_weight = 10;

        let items = [
            Item {
                weight: 2,

                value: 20,
            },
            Item {
                weight: 2,

                value: 20,
            },
            Item {
                weight: 2,

                value: 20,
            },
            Item {
                weight: 2,

                value: 20,
            },
            Item {
                weight: 10,

                value: 50,
            },
        ];

        assert_eq!(maximum_value(max_weight, &items), 80);
    }

    #[test]
    fn test_8_items() {
        let max_weight = 104;

        let items = [
            Item {
                weight: 25,

                value: 350,
            },
            Item {
                weight: 35,

                value: 400,
            },
            Item {
                weight: 45,

                value: 450,
            },
            Item {
                weight: 5,

                value: 20,
            },
            Item {
                weight: 25,

                value: 70,
            },
            Item {
                weight: 3,

                value: 8,
            },
            Item {
                weight: 2,

                value: 5,
            },
            Item {
                weight: 2,

                value: 5,
            },
        ];

        assert_eq!(maximum_value(max_weight, &items), 900);
    }

    #[test]
    fn test_15_items() {
        let max_weight = 750;

        let items = [
            Item {
                weight: 70,

                value: 135,
            },
            Item {
                weight: 73,

                value: 139,
            },
            Item {
                weight: 77,

                value: 149,
            },
            Item {
                weight: 80,

                value: 150,
            },
            Item {
                weight: 82,

                value: 156,
            },
            Item {
                weight: 87,

                value: 163,
            },
            Item {
                weight: 90,

                value: 173,
            },
            Item {
                weight: 94,

                value: 184,
            },
            Item {
                weight: 98,

                value: 192,
            },
            Item {
                weight: 106,

                value: 201,
            },
            Item {
                weight: 110,

                value: 210,
            },
            Item {
                weight: 113,

                value: 214,
            },
            Item {
                weight: 115,

                value: 221,
            },
            Item {
                weight: 118,

                value: 229,
            },
            Item {
                weight: 120,

                value: 240,
            },
        ];

        assert_eq!(maximum_value(max_weight, &items), 1458);
    }
}
