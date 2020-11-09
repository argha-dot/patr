use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
	// Can only contain a-z, A-Z, 0-9, . and _. Cannot begin with a . (github rules, basically)
	static ref USERNAME_REGEX: Regex = Regex::new("[a-zA-Z0-9_]+[a-zA-Z0-9_\\.]*").unwrap();
	// Email regex: https://stackoverflow.com/a/201378
	static ref EMAIL_REGEX: Regex = Regex::new("(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|\"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*\")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\\[(?:(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9]))\\.){3}(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9])|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\\])").unwrap();
	// Needs to have a '+' to start with, and be between 9-15 numbers after that
	static ref PHONE_NUMBER_REGEX: Regex = Regex::new("\\+?[0-9]{9,15}$").unwrap();
	// Needs to have at least 1 a-z, 1 A-Z, 1 0-9 and a special character
	//static ref PASSWORD_REGEX: Regex = Regex::new("^(?=.*[0-9])(?=.*[a-z])(?=.*[A-Z])(?=.*[@#$%^&-+=()])(?=\\S+$).{8,}$").unwrap();
}

pub fn is_username_valid(username: &str) -> bool {
	username.len() <= 100 && USERNAME_REGEX.is_match(username)
}

pub fn is_email_valid(email: &str) -> bool {
	email.len() <= 320 && EMAIL_REGEX.is_match(email)
}

pub fn is_password_valid(password: &str) -> bool {
	let mut has_lower_case = false;
	let mut has_upper_case = false;
	let mut has_number = false;
	let mut has_special_character = false;
	password.chars().for_each(|ch| {
		if ch.is_ascii_lowercase() {
			has_lower_case = true;
		}
		if ch.is_ascii_uppercase() {
			has_upper_case = true;
		}
		if ch.is_numeric() {
			has_number = true;
		}
		if "@#$%^&-+=()".contains(ch) {
			has_special_character = true
		}
	});
	password.len() >= 8 &&
		has_lower_case &&
		has_upper_case &&
		has_number &&
		has_special_character
}

#[allow(dead_code)]
pub fn is_phone_number_valid(phone_number: &str) -> bool {
	PHONE_NUMBER_REGEX.is_match(phone_number)
}
