static VECTORS_X: [isize; 4] = [0, 1, 0, -1];
static VECTORS_Y: [isize; 4] = [1, 0, -1, 0];

pub fn spiral_matrix(size: usize) -> Vec<Vec<u32>> {
    let mut result: Vec<Vec<u32>> = vec![vec![0; size]; size];
    if size == 0 {
        return result;
    }

    let mut x = 0isize;
    let mut y = -1isize;
    let mut v = 1u32;
    for i in 0..(size + size - 1) {
        for _ in 0..((size + size - i) / 2) {
            x += VECTORS_X[i % 4];
            y += VECTORS_Y[i % 4];
            result[x as usize][y as usize] = v;
            v += 1;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::spiral_matrix::*;

    #[test]
    fn empty_spiral() {
        let expected: Vec<Vec<u32>> = Vec::new();

        assert_eq!(spiral_matrix(0), expected);
    }

    #[test]
    fn size_one_spiral() {
        let expected: Vec<Vec<u32>> = vec![vec![1]];

        assert_eq!(spiral_matrix(1), expected);
    }

    #[test]
    fn size_two_spiral() {
        let expected: Vec<Vec<u32>> = vec![vec![1, 2], vec![4, 3]];

        assert_eq!(spiral_matrix(2), expected);
    }

    #[test]
    fn size_three_spiral() {
        #[rustfmt::skip]
    let expected: Vec<Vec<u32>> = vec![
        vec![1, 2, 3],
        vec![8, 9, 4],
        vec![7, 6, 5],
    ];
        assert_eq!(spiral_matrix(3), expected);
    }
    #[test]
    fn size_four_spiral() {
        let expected: Vec<Vec<u32>> = vec![
            vec![1, 2, 3, 4],
            vec![12, 13, 14, 5],
            vec![11, 16, 15, 6],
            vec![10, 9, 8, 7],
        ];
        assert_eq!(spiral_matrix(4), expected);
    }
    #[test]
    fn size_five_spiral() {
        let expected: Vec<Vec<u32>> = vec![
            vec![1, 2, 3, 4, 5],
            vec![16, 17, 18, 19, 6],
            vec![15, 24, 25, 20, 7],
            vec![14, 23, 22, 21, 8],
            vec![13, 12, 11, 10, 9],
        ];
        assert_eq!(spiral_matrix(5), expected);
    }
}
