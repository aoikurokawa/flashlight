use num::Zero;

#[derive(Debug, Clone)]
pub struct Triangle<T> {
    a: T,
    b: T,
    c: T,
}

impl<T> Triangle<T>
where
    T: Clone + Copy + PartialEq + Zero + PartialOrd,
{
    pub fn build(sides: [T; 3]) -> Option<Triangle<T>> {
        for side in sides.iter() {
            if side.is_zero() {
                return None;
            }
        }

        if sides[0] + sides[1] > sides[2]
            && sides[1] + sides[2] > sides[0]
            && sides[1] + sides[2] > sides[1]
        {
            Some(Self {
                a: sides[0],
                b: sides[1],
                c: sides[2],
            })
        } else {
            None
        }
    }

    // three sides the same length
    pub fn is_equilateral(&self) -> bool {
        self.a == self.b && self.b == self.c
    }

    // all sides of different lengths
    pub fn is_scalene(&self) -> bool {
        !self.is_equilateral() && !self.is_isosceles()
    }

    // at least two sides the same length
    pub fn is_isosceles(&self) -> bool {
        !self.is_equilateral() && (self.a == self.b || self.b == self.c || self.a == self.c)
    }
}

#[cfg(test)]
mod tests {
    use crate::triangle::*;

    // pass
    #[test]
    fn positive_length_sides_are_ok() {
        let sides = [2, 2, 2];

        let triangle = Triangle::build(sides);

        assert!(triangle.is_some());
    }

    // pass
    #[test]
    fn zero_length_sides_are_illegal() {
        let sides = [0, 0, 0];

        let triangle = Triangle::build(sides);

        assert!(triangle.is_none());
    }

    // pass
    #[test]
    fn one_length_zero_side_first() {
        let sides = [0, 2, 2];

        let triangle = Triangle::build(sides);

        assert!(triangle.is_none());
    }

    // pass
    #[test]
    fn one_length_zero_side_second() {
        let sides = [2, 0, 2];

        let triangle = Triangle::build(sides);

        assert!(triangle.is_none());
    }

    // pass
    #[test]
    fn one_length_zero_side_third() {
        let sides = [2, 2, 0];

        let triangle = Triangle::build(sides);

        assert!(triangle.is_none());
    }

    // pass
    #[test]
    fn equilateral_triangles_have_equal_sides() {
        let sides = [2, 2, 2];

        let triangle = Triangle::build(sides).unwrap();

        assert!(triangle.is_equilateral());

        assert!(!triangle.is_scalene());
    }

    // pass
    #[test]
    fn larger_equilateral_triangles_have_equal_sides() {
        let sides = [10, 10, 10];

        let triangle = Triangle::build(sides).unwrap();

        assert!(triangle.is_equilateral());

        assert!(!triangle.is_scalene());
    }

    #[test]
    fn isosceles_triangles_have_two_equal_sides_one() {
        let sides = [3, 4, 4];

        let triangle = Triangle::build(sides).unwrap();

        assert!(!triangle.is_equilateral());

        assert!(triangle.is_isosceles());

        assert!(!triangle.is_scalene());
    }

    #[test]
    fn isosceles_triangles_have_two_equal_sides_two() {
        let sides = [4, 4, 3];

        let triangle = Triangle::build(sides).unwrap();

        assert!(!triangle.is_equilateral());

        assert!(triangle.is_isosceles());

        assert!(!triangle.is_scalene());
    }

    #[test]
    fn isosceles_triangles_have_two_equal_sides_three() {
        let sides = [4, 3, 4];

        let triangle = Triangle::build(sides).unwrap();

        assert!(!triangle.is_equilateral());

        assert!(triangle.is_isosceles());

        assert!(!triangle.is_scalene());
    }

    #[test]
    fn isosceles_triangles_have_two_equal_sides_four() {
        let sides = [4, 7, 4];

        let triangle = Triangle::build(sides).unwrap();

        assert!(!triangle.is_equilateral());

        assert!(triangle.is_isosceles());

        assert!(!triangle.is_scalene());
    }

    #[test]
    fn scalene_triangle_has_no_equal_sides_one() {
        let sides = [3, 4, 5];

        let triangle = Triangle::build(sides).unwrap();

        assert!(!triangle.is_equilateral());

        assert!(!triangle.is_isosceles());

        assert!(triangle.is_scalene());
    }

    #[test]
    fn scalene_triangle_has_no_equal_sides_two() {
        let sides = [5, 4, 6];

        let triangle = Triangle::build(sides).unwrap();

        assert!(!triangle.is_equilateral());

        assert!(!triangle.is_isosceles());

        assert!(triangle.is_scalene());
    }

    #[test]
    fn scalene_triangle_has_no_equal_sides_three() {
        let sides = [10, 11, 12];

        let triangle = Triangle::build(sides).unwrap();

        assert!(!triangle.is_equilateral());

        assert!(!triangle.is_isosceles());

        assert!(triangle.is_scalene());
    }

    #[test]
    fn scalene_triangle_has_no_equal_sides_four() {
        let sides = [5, 4, 2];

        let triangle = Triangle::build(sides).unwrap();

        assert!(!triangle.is_equilateral());

        assert!(!triangle.is_isosceles());

        assert!(triangle.is_scalene());
    }

    #[test]
    fn sum_of_two_sides_must_equal_or_exceed_the_remaining_side_one() {
        let sides = [7, 3, 2];

        let triangle = Triangle::build(sides);

        assert!(triangle.is_none());
    }

    #[test]
    fn sum_of_two_sides_must_equal_or_exceed_the_remaining_side_two() {
        let sides = [1, 1, 3];

        let triangle = Triangle::build(sides);

        assert!(triangle.is_none());
    }

    #[test]
    fn scalene_triangle_with_floating_point_sides() {
        let sides = [0.4, 0.6, 0.3];

        let triangle = Triangle::build(sides).unwrap();

        assert!(!triangle.is_equilateral());

        assert!(!triangle.is_isosceles());

        assert!(triangle.is_scalene());
    }

    #[test]
    fn equilateral_triangles_with_floating_point_sides() {
        let sides = [0.2, 0.2, 0.2];

        let triangle = Triangle::build(sides).unwrap();

        assert!(triangle.is_equilateral());

        assert!(!triangle.is_scalene());
    }

    #[test]
    fn isosceles_triangle_with_floating_point_sides() {
        let sides = [0.3, 0.4, 0.4];

        let triangle = Triangle::build(sides).unwrap();

        assert!(!triangle.is_equilateral());

        assert!(triangle.is_isosceles());

        assert!(!triangle.is_scalene());
    }

    #[test]
    fn invalid_triangle_with_floating_point_sides_one() {
        let sides = [0.0, 0.4, 0.3];

        let triangle = Triangle::build(sides);

        assert!(triangle.is_none());
    }

    #[test]
    fn invalid_triangle_with_floating_point_sides_two() {
        let sides = [0.1, 0.3, 0.5];

        let triangle = Triangle::build(sides);

        assert!(triangle.is_none());
    }
}
