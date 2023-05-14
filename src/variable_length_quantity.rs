#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    IncompleteNumber,
    Overflow,
}

/// Convert a list of numbers to a stream of bytes encoded with variable length encoding.
pub fn to_bytes(values: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for mut v in values.iter().cloned().rev() {
        bytes.push((v & 0x7F) as u8);
        v >>= 7;
        while v != 0 {
            bytes.push((v & 0x7F | 0x80) as u8);
            v >>= 7;
        }
    }
    bytes.into_iter().rev().collect()
}

/// Given a stream of bytes, extract all numbers which are encoded in there.
pub fn from_bytes(bytes: &[u8]) -> Result<Vec<u32>, Error> {
    if let Some(last) = bytes.last() {
        if last & 0x80 != 0 {
            return Err(Error::IncompleteNumber);
        }
    }

    let mut numbers = Vec::new();
    let mut number: u32 = 0;
    for &byte in bytes.iter() {
        if number.leading_zeros() < 7 {
            return Err(Error::Overflow);
        }
        number = number << 7 | (byte as u32 & 0x7f);
        if byte & 0x80 == 0 {
            numbers.push(number);
            number = 0;
        }
    }

    Ok(numbers)
}

#[cfg(test)]
mod tests {
    use crate::variable_length_quantity as vlq;

    #[test]
    fn to_single_byte() {
        assert_eq!(&[0x00], vlq::to_bytes(&[0x00]).as_slice());

        assert_eq!(&[0x40], vlq::to_bytes(&[0x40]).as_slice());

        assert_eq!(&[0x7f], vlq::to_bytes(&[0x7f]).as_slice());
    }

    #[test]
    fn to_double_byte() {
        assert_eq!(&[0x81, 0x00], vlq::to_bytes(&[0x80]).as_slice());

        assert_eq!(&[0xc0, 0x00], vlq::to_bytes(&[0x2000]).as_slice());

        assert_eq!(&[0xff, 0x7f], vlq::to_bytes(&[0x3fff]).as_slice());
    }

    #[test]
    fn to_triple_byte() {
        assert_eq!(&[0x81, 0x80, 0x00], vlq::to_bytes(&[0x4000]).as_slice());

        assert_eq!(&[0xc0, 0x80, 0x00], vlq::to_bytes(&[0x10_0000]).as_slice());

        assert_eq!(&[0xff, 0xff, 0x7f], vlq::to_bytes(&[0x1f_ffff]).as_slice());
    }

    #[test]
    fn to_quadruple_byte() {
        assert_eq!(
            &[0x81, 0x80, 0x80, 0x00],
            vlq::to_bytes(&[0x20_0000]).as_slice()
        );

        assert_eq!(
            &[0xc0, 0x80, 0x80, 0x00],
            vlq::to_bytes(&[0x0800_0000]).as_slice()
        );

        assert_eq!(
            &[0xff, 0xff, 0xff, 0x7f],
            vlq::to_bytes(&[0x0fff_ffff]).as_slice()
        );
    }

    #[test]
    fn to_quintuple_byte() {
        assert_eq!(
            &[0x81, 0x80, 0x80, 0x80, 0x00],
            vlq::to_bytes(&[0x1000_0000]).as_slice()
        );

        assert_eq!(
            &[0x8f, 0xf8, 0x80, 0x80, 0x00],
            vlq::to_bytes(&[0xff00_0000]).as_slice()
        );

        assert_eq!(
            &[0x8f, 0xff, 0xff, 0xff, 0x7f],
            vlq::to_bytes(&[0xffff_ffff]).as_slice()
        );
    }

    #[test]
    fn from_bytes() {
        assert_eq!(Ok(vec![0x7f]), vlq::from_bytes(&[0x7f]));

        assert_eq!(Ok(vec![0x2000]), vlq::from_bytes(&[0xc0, 0x00]));

        assert_eq!(Ok(vec![0x1f_ffff]), vlq::from_bytes(&[0xff, 0xff, 0x7f]));

        assert_eq!(
            Ok(vec![0x20_0000]),
            vlq::from_bytes(&[0x81, 0x80, 0x80, 0x00])
        );

        assert_eq!(
            Ok(vec![0xffff_ffff]),
            vlq::from_bytes(&[0x8f, 0xff, 0xff, 0xff, 0x7f])
        );
    }

    #[test]
    fn to_bytes_multiple_values() {
        assert_eq!(&[0x40, 0x7f], vlq::to_bytes(&[0x40, 0x7f]).as_slice());

        assert_eq!(
            &[0x81, 0x80, 0x00, 0xc8, 0xe8, 0x56],
            vlq::to_bytes(&[0x4000, 0x12_3456]).as_slice()
        );

        assert_eq!(
            &[
                0xc0, 0x00, 0xc8, 0xe8, 0x56, 0xff, 0xff, 0xff, 0x7f, 0x00, 0xff, 0x7f, 0x81, 0x80,
                0x00,
            ],
            vlq::to_bytes(&[0x2000, 0x12_3456, 0x0fff_ffff, 0x00, 0x3fff, 0x4000]).as_slice()
        );
    }

    #[test]
    fn from_bytes_multiple_values() {
        assert_eq!(
            Ok(vec![0x2000, 0x12_3456, 0x0fff_ffff, 0x00, 0x3fff, 0x4000]),
            vlq::from_bytes(&[
                0xc0, 0x00, 0xc8, 0xe8, 0x56, 0xff, 0xff, 0xff, 0x7f, 0x00, 0xff, 0x7f, 0x81, 0x80,
                0x00,
            ])
        );
    }

    #[test]
    fn incomplete_byte_sequence() {
        assert_eq!(Err(vlq::Error::IncompleteNumber), vlq::from_bytes(&[0xff]));
    }

    #[test]
    fn zero_incomplete_byte_sequence() {
        assert_eq!(Err(vlq::Error::IncompleteNumber), vlq::from_bytes(&[0x80]));
    }

    #[test]
    fn overflow_u32() {
        assert_eq!(
            Err(vlq::Error::Overflow),
            vlq::from_bytes(&[0xff, 0xff, 0xff, 0xff, 0x7f])
        );
    }

    #[test]
    fn chained_execution_is_identity() {
        let test = &[0xf2, 0xf6, 0x96, 0x9c, 0x3b, 0x39, 0x2e, 0x30, 0xb3, 0x24];

        assert_eq!(
            Ok(Vec::from(&test[..])),
            vlq::from_bytes(&vlq::to_bytes(test))
        );
    }
}
