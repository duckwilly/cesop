use rand::Rng;

pub fn random_digits<R: Rng + ?Sized>(rng: &mut R, len: usize) -> String {
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        let digit = rng.gen_range(0..10);
        out.push(char::from(b'0' + digit as u8));
    }
    out
}

pub fn random_alphanum_upper<R: Rng + ?Sized>(rng: &mut R, len: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        let idx = rng.gen_range(0..CHARSET.len());
        out.push(char::from(CHARSET[idx]));
    }
    out
}

pub fn random_upper_letters<R: Rng + ?Sized>(rng: &mut R, len: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        let idx = rng.gen_range(0..CHARSET.len());
        out.push(char::from(CHARSET[idx]));
    }
    out
}

pub fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_dash = false;
    for ch in input.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

pub fn format_amount(value: f64) -> String {
    format!("{:.2}", value)
}

pub fn iban_check_digits(country: &str, bban: &str) -> Result<String, String> {
    if country.len() != 2 {
        return Err("IBAN country code must be 2 letters".to_string());
    }
    let mut remainder: u32 = 0;
    let combined = format!("{}{}00", bban, country);
    for ch in combined.chars() {
        let chunk = if ch.is_ascii_digit() {
            ch.to_string()
        } else if ch.is_ascii_alphabetic() {
            let val = ch.to_ascii_uppercase() as u32 - 'A' as u32 + 10;
            val.to_string()
        } else {
            return Err("IBAN contains invalid character".to_string());
        };
        for digit in chunk.chars() {
            let d = digit.to_digit(10).ok_or_else(|| "invalid digit".to_string())?;
            remainder = (remainder * 10 + d) % 97;
        }
    }

    let check = 98 - remainder;
    Ok(format!("{:02}", check))
}
