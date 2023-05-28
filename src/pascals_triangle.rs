pub struct PascalsTriangle {
    row: u32,
}

impl PascalsTriangle {
    pub fn new(row_count: u32) -> Self {
        Self { row: row_count }
    }

    pub fn rows(&self) -> Vec<Vec<u32>> {
        let mut triangles: Vec<Vec<u32>> = Vec::new();
        if self.row == 0 {
            return triangles;
        }

        triangles.push(vec![1]);

        while triangles.len() < self.row as usize {
            let mut current_row = vec![1];
            let previous_row = triangles.pop().unwrap();

            for (idx, num) in previous_row.iter().enumerate() {
                if idx == 0 {
                    continue;
                }

                let prev_num = match idx.checked_sub(1) {
                    Some(sub_idx) => previous_row.get(sub_idx).unwrap_or(&0),
                    None => &0,
                };
                current_row.push(*num + prev_num);
            }

            triangles.push(previous_row);

            current_row.push(1);
            triangles.push(current_row);
        }

        triangles
    }
}

#[cfg(test)]
mod tests {
    use crate::pascals_triangle::*;

    #[test]
    fn no_rows() {
        let pt = PascalsTriangle::new(0);

        let expected: Vec<Vec<u32>> = Vec::new();

        assert_eq!(expected, pt.rows());
    }

    #[test]
    fn one_row() {
        let pt = PascalsTriangle::new(1);

        let expected: Vec<Vec<u32>> = vec![vec![1]];

        assert_eq!(expected, pt.rows());
    }

    #[test]
    fn two_rows() {
        let pt = PascalsTriangle::new(2);

        let expected: Vec<Vec<u32>> = vec![vec![1], vec![1, 1]];

        assert_eq!(expected, pt.rows());
    }

    #[test]
    fn three_rows() {
        let pt = PascalsTriangle::new(3);

        let expected: Vec<Vec<u32>> = vec![vec![1], vec![1, 1], vec![1, 2, 1]];

        assert_eq!(expected, pt.rows());
    }

    #[test]
    fn last_of_four_rows() {
        let pt = PascalsTriangle::new(4);

        let expected: Vec<u32> = vec![1, 3, 3, 1];

        assert_eq!(Some(expected), pt.rows().pop());
    }

    #[test]
    fn five_rows() {
        let pt = PascalsTriangle::new(5);

        let expected: Vec<Vec<u32>> = vec![
            vec![1],
            vec![1, 1],
            vec![1, 2, 1],
            vec![1, 3, 3, 1],
            vec![1, 4, 6, 4, 1],
        ];

        assert_eq!(expected, pt.rows());
    }

    #[test]
    fn six_rows() {
        let pt = PascalsTriangle::new(6);

        let expected: Vec<Vec<u32>> = vec![
            vec![1],
            vec![1, 1],
            vec![1, 2, 1],
            vec![1, 3, 3, 1],
            vec![1, 4, 6, 4, 1],
            vec![1, 5, 10, 10, 5, 1],
        ];

        assert_eq!(expected, pt.rows());
    }

    #[test]
    fn seven_rows() {
        let pt = PascalsTriangle::new(7);

        let expected: Vec<Vec<u32>> = vec![
            vec![1],
            vec![1, 1],
            vec![1, 2, 1],
            vec![1, 3, 3, 1],
            vec![1, 4, 6, 4, 1],
            vec![1, 5, 10, 10, 5, 1],
            vec![1, 6, 15, 20, 15, 6, 1],
        ];

        assert_eq!(expected, pt.rows());
    }

    #[test]
    fn ten_rows() {
        let pt = PascalsTriangle::new(10);

        let expected: Vec<Vec<u32>> = vec![
            vec![1],
            vec![1, 1],
            vec![1, 2, 1],
            vec![1, 3, 3, 1],
            vec![1, 4, 6, 4, 1],
            vec![1, 5, 10, 10, 5, 1],
            vec![1, 6, 15, 20, 15, 6, 1],
            vec![1, 7, 21, 35, 35, 21, 7, 1],
            vec![1, 8, 28, 56, 70, 56, 28, 8, 1],
            vec![1, 9, 36, 84, 126, 126, 84, 36, 9, 1],
        ];

        assert_eq!(expected, pt.rows());
    }
}
