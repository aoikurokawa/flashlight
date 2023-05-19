// The code below is a stub. Just enough to satisfy the compiler.
// In order to pass the tests you can add-to or change any of this code.

#[derive(PartialEq, Eq, Debug)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

pub struct Robot {
    x: i32,
    y: i32,
    direction: Direction,
}

impl Robot {
    pub fn new(x: i32, y: i32, d: Direction) -> Self {
        Self { x, y, direction: d }
    }

    #[must_use]
    pub fn turn_right(self) -> Self {
        let direction = match self.direction {
            Direction::North => Direction::East,
            Direction::East => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
        };

        Self {
            x: self.x,
            y: self.y,
            direction,
        }
    }

    #[must_use]
    pub fn turn_left(mut self) -> Self {
        match self.direction {
            Direction::North => self.direction = Direction::West,
            Direction::East => self.direction = Direction::North,
            Direction::South => self.direction = Direction::East,
            Direction::West => self.direction = Direction::South,
        }

        Self { ..self }
    }

    #[must_use]
    pub fn advance(mut self) -> Self {
        match self.direction {
            Direction::North => self.y += 1,
            Direction::South => self.y -= 1,
            Direction::East => self.x += 1,
            Direction::West => self.x -= 1,
        }

        Self { ..self }
    }

    #[must_use]
    pub fn instructions(mut self, instructions: &str) -> Self {
        for ch in instructions.chars() {
            match ch {
                'A' => self = self.advance(),
                'R' => self = self.turn_right(),
                'L' => self = self.turn_left(),
                _ => {}
            }
        }

        Self { ..self }
    }

    pub fn position(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn direction(&self) -> &Direction {
        &self.direction
    }
}

#[cfg(test)]
mod tests {
    use crate::robot_simulator::*;

    #[test]
    fn robots_are_created_with_position_and_direction() {
        let robot = Robot::new(0, 0, Direction::North);

        assert_eq!((0, 0), robot.position());

        assert_eq!(&Direction::North, robot.direction());
    }

    #[test]
    fn positions_can_be_negative() {
        let robot = Robot::new(-1, -1, Direction::South);

        assert_eq!((-1, -1), robot.position());

        assert_eq!(&Direction::South, robot.direction());
    }

    #[test]
    fn turning_right_does_not_change_position() {
        let robot = Robot::new(0, 0, Direction::North).turn_right();

        assert_eq!((0, 0), robot.position());
    }

    #[test]
    fn turning_right_from_north_points_the_robot_east() {
        let robot = Robot::new(0, 0, Direction::North).turn_right();

        assert_eq!(&Direction::East, robot.direction());
    }

    #[test]
    fn turning_right_from_east_points_the_robot_south() {
        let robot = Robot::new(0, 0, Direction::East).turn_right();

        assert_eq!(&Direction::South, robot.direction());
    }

    #[test]
    fn turning_right_from_south_points_the_robot_west() {
        let robot = Robot::new(0, 0, Direction::South).turn_right();

        assert_eq!(&Direction::West, robot.direction());
    }

    #[test]
    fn turning_right_from_west_points_the_robot_north() {
        let robot = Robot::new(0, 0, Direction::West).turn_right();

        assert_eq!(&Direction::North, robot.direction());
    }

    #[test]
    fn turning_left_does_not_change_position() {
        let robot = Robot::new(0, 0, Direction::North).turn_left();

        assert_eq!((0, 0), robot.position());
    }

    #[test]
    fn turning_left_from_north_points_the_robot_west() {
        let robot = Robot::new(0, 0, Direction::North).turn_left();

        assert_eq!(&Direction::West, robot.direction());
    }

    #[test]
    fn turning_left_from_west_points_the_robot_south() {
        let robot = Robot::new(0, 0, Direction::West).turn_left();

        assert_eq!(&Direction::South, robot.direction());
    }

    #[test]
    fn turning_left_from_south_points_the_robot_east() {
        let robot = Robot::new(0, 0, Direction::South).turn_left();

        assert_eq!(&Direction::East, robot.direction());
    }

    #[test]
    fn turning_left_from_east_points_the_robot_north() {
        let robot = Robot::new(0, 0, Direction::East).turn_left();

        assert_eq!(&Direction::North, robot.direction());
    }

    #[test]
    fn advance_does_not_change_the_direction() {
        let robot = Robot::new(0, 0, Direction::North).advance();

        assert_eq!(&Direction::North, robot.direction());
    }

    #[test]
    fn advance_increases_the_y_coordinate_by_one_when_facing_north() {
        let robot = Robot::new(0, 0, Direction::North).advance();

        assert_eq!((0, 1), robot.position());
    }

    #[test]
    fn advance_decreases_the_y_coordinate_by_one_when_facing_south() {
        let robot = Robot::new(0, 0, Direction::South).advance();

        assert_eq!((0, -1), robot.position());
    }

    #[test]
    fn advance_increases_the_x_coordinate_by_one_when_facing_east() {
        let robot = Robot::new(0, 0, Direction::East).advance();

        assert_eq!((1, 0), robot.position());
    }

    #[test]
    fn advance_decreases_the_x_coordinate_by_one_when_facing_west() {
        let robot = Robot::new(0, 0, Direction::West).advance();

        assert_eq!((-1, 0), robot.position());
    }

    #[test]
    fn follow_instructions_to_move_west_and_north() {
        let robot = Robot::new(0, 0, Direction::North).instructions("LAAARALA");

        assert_eq!((-4, 1), robot.position());

        assert_eq!(&Direction::West, robot.direction());
    }

    #[test]
    fn follow_instructions_to_move_west_and_south() {
        let robot = Robot::new(2, -7, Direction::East).instructions("RRAAAAALA");

        assert_eq!((-3, -8), robot.position());

        assert_eq!(&Direction::South, robot.direction());
    }

    #[test]
    fn follow_instructions_to_move_east_and_north() {
        let robot = Robot::new(8, 4, Direction::South).instructions("LAAARRRALLLL");

        assert_eq!((11, 5), robot.position());

        assert_eq!(&Direction::North, robot.direction());
    }
}
