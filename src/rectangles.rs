pub fn count(lines: &[&str]) -> u32 {
    let v: Vec<Vec<char>> = lines.iter().map(|row| row.chars().collect()).collect();

    let mut count = 0;
    for top in 0..v.len() {
        for left in 0..v[top].len() {
            if v[top][left] == '+' {
                for right in left + 1..v[top].len() {
                    if v[top][right] == '+' {
                        for (idx, _side) in v.iter().skip(top + 1).enumerate() {
                            match (v[idx][left], v[idx][right]) {
                                ('+', '+') => {
                                    if v[idx][left + 1..right]
                                        .iter()
                                        .all(|c| *c == '-' || *c == '+')
                                    {
                                        count += 1;
                                    } else {
                                        continue;
                                    }
                                }
                                ('+', '|') => continue,
                                ('|', '+') => continue,
                                ('|', '|') => continue,
                                (_, _) => break,
                            }
                        }
                    } else if v[top][right] != '-' {
                        break;
                    }
                }
            }
        }
    }

    count
}
