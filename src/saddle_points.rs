pub fn find_saddle_points(input: &[Vec<u64>]) -> Vec<(usize, usize)> {
    if input.is_empty() {
        return Vec::new();
    }

    let ncols = input[0].len();
    if ncols == 0 {
        return Vec::new();
    }

    let row_maxes: Vec<u64> = input
        .iter()
        .map(|v| v.iter().copied().max().unwrap())
        .collect();
    let col_mins: Vec<u64> = (0..ncols)
        .map(|i| input.iter().map(|v| v[i]).min().unwrap())
        .collect();
    let mut points: Vec<(usize, usize)> = Vec::new();
    for (j, v) in input.iter().enumerate() {
        for (i, &x) in v.iter().enumerate() {
            if (x == row_maxes[j]) && (x == col_mins[i]) {
                points.push((j, i));
            }
        }
    }

    points
}

#[cfg(test)]
mod tests {
    use crate::saddle_points::find_saddle_points;
    // We don't care about order
    fn find_sorted_saddle_points(input: &[Vec<u64>]) -> Vec<(usize, usize)> {
        let mut result = crate::saddle_points::find_saddle_points(input);
        result.sort_unstable();
        result
    }
    #[test]
    fn identify_single_saddle_point() {
        let input = vec![vec![9, 8, 7], vec![5, 3, 2], vec![6, 6, 7]];
        assert_eq!(vec![(1, 0)], find_saddle_points(&input));
    }
    #[test]
    fn identify_empty_matrix() {
        let input = vec![vec![], vec![], vec![]];
        let expected: Vec<(usize, usize)> = Vec::new();
        assert_eq!(expected, find_saddle_points(&input));
    }
    #[test]
    fn identify_lack_of_saddle_point() {
        let input = vec![vec![1, 2, 3], vec![3, 1, 2], vec![2, 3, 1]];
        let expected: Vec<(usize, usize)> = Vec::new();
        assert_eq!(expected, find_saddle_points(&input));
    }
    #[test]
    fn multiple_saddle_points_in_col() {
        let input = vec![vec![4, 5, 4], vec![3, 5, 5], vec![1, 5, 4]];
        assert_eq!(
            vec![(0, 1), (1, 1), (2, 1)],
            find_sorted_saddle_points(&input)
        );
    }
    #[test]
    fn multiple_saddle_points_in_row() {
        let input = vec![vec![6, 7, 8], vec![5, 5, 5], vec![7, 5, 6]];
        assert_eq!(
            vec![(1, 0), (1, 1), (1, 2)],
            find_sorted_saddle_points(&input)
        );
    }
    #[test]
    fn identify_bottom_right_saddle_point() {
        let input = vec![vec![8, 7, 9], vec![6, 7, 6], vec![3, 2, 5]];
        assert_eq!(vec![(2, 2)], find_saddle_points(&input));
    }
    // track specific as of v1.3
    #[test]
    fn non_square_matrix_high() {
        let input = vec![vec![1, 5], vec![3, 6], vec![2, 7], vec![3, 8]];
        assert_eq!(vec![(0, 1)], find_saddle_points(&input));
    }
    #[test]
    fn non_square_matrix_wide() {
        let input = vec![vec![3, 1, 3], vec![3, 2, 4]];
        assert_eq!(vec![(0, 0), (0, 2)], find_sorted_saddle_points(&input));
    }
    #[test]
    fn single_column_matrix() {
        let input = vec![vec![2], vec![1], vec![4], vec![1]];
        assert_eq!(vec![(1, 0), (3, 0)], find_sorted_saddle_points(&input));
    }
    #[test]
    fn single_row_matrix() {
        let input = vec![vec![2, 5, 3, 5]];
        assert_eq!(vec![(0, 1), (0, 3)], find_sorted_saddle_points(&input));
    }
    #[test]
    fn identify_all_saddle_points() {
        let input = vec![vec![5, 5, 5], vec![5, 5, 5], vec![5, 5, 5]];
        assert_eq!(
            vec![
                (0, 0),
                (0, 1),
                (0, 2),
                (1, 0),
                (1, 1),
                (1, 2),
                (2, 0),
                (2, 1),
                (2, 2)
            ],
            find_sorted_saddle_points(&input)
        );
    }
}
