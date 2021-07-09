use lettre::message::Mailbox;
use serde::Serialize;

use crate::{models::EmailTemplate, utils::Error};

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/user-sign-up/template.json"]
struct UserSignUpVerificationEmail {
	otp: String,
}

/// # Description
/// This function is used to email the otp to user for account verification
///
/// # Arguments
/// * `email` - Represents an email address with an optional name for the
///   sender/recipient.
/// More info here: [`Mailbox`]
/// * `otp` - a string containing One Time Password to be sent to the user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Mailbox`]: Mailbox
pub async fn send_user_verification_otp(
	email: Mailbox,
	otp: &str,
) -> Result<(), Error> {
	send_email(
		UserSignUpVerificationEmail {
			otp: otp.to_string(),
		},
		email,
		None,
		"",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/forgot-password/template.json"]
struct ForgotPasswordEmail {
	otp: String,
}

/// # Description
/// This function is used to email the otp to user for verifiying change in
/// password incase the user forgets the password
///
/// # Arguments
/// * `email` - Represents an email address with an optional name for the
///   sender/recipient.
/// More info here: [`Mailbox`]
/// * `otp` - a string containing One Time Password to be sent to the user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Mailbox`]: Mailbox
pub async fn send_forgot_password_otp(
	email: Mailbox,
	otp: &str,
) -> Result<(), Error> {
	send_email(
		ForgotPasswordEmail {
			otp: otp.to_string(),
		},
		email,
		None,
		"",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/password-reset-notification/template.json"]
struct PasswordResetEmail {}

/// # Description
/// This function is used to send the password reset notification
///
/// # Arguments
/// * `email` - Represents an email address with an optional name for the
///   sender/recipient.
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Mailbox`]: Mailbox
pub async fn send_user_reset_password_notification(
	email: Mailbox,
) -> Result<(), Error> {
	send_email(PasswordResetEmail {}, email, None, "").await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/password-changed-notification/template.json"]
struct PasswordChangedEmail {}

/// # Description
/// This function is used to send the password changed notification
///
/// # Arguments
/// * `email` - Represents an email address with an optional name for the
///   sender/recipient.
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
pub async fn send_password_changed_notification(
	email: Mailbox,
) -> Result<(), Error> {
	send_email(PasswordChangedEmail {}, email, None, "").await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/sign-up-completed/template.json"]
struct SignUpCompletedEmail {}

/// # Description
/// This function is used to send the sign up complete notification
///
/// # Arguments
/// * `email` - Represents an email address with an optional name for the
///   sender/recipient.
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
pub async fn send_sign_up_completed_email(email: Mailbox) -> Result<(), Error> {
	send_email(SignUpCompletedEmail {}, email, None, "").await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/backup-email-notification/template.json"]
struct BackupNotificationEmail {}

/// # Description
/// This function is used to send the registration info to back up email of the
/// user
///
/// # Arguments
/// * `email` - Represents an email address with an optional name for the
///   sender/recipient.
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
pub async fn send_backup_registration_mail(
	email: Mailbox,
) -> Result<(), Error> {
	send_email(BackupNotificationEmail {}, email, None, "").await
}

/// # Description
/// This function is used to send the email to a recipient
///
/// # Arguments
/// * `body` - body of the mail of the type [`TEmail`]
/// * `to` - recipient's email address of type [`Mailbox`]
/// * `reply_to` - An Option<Mailbox> containing instance of [`Mailbox`]
///   containing email of recipient
/// to be replied or `None`
/// * `subject` - a string containing subject of the email
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// errors
///
/// [`TEmail`]: TEmail
#[cfg(not(debug_assertions))]
async fn send_email<TEmail>(
	body: TEmail,
	to: Mailbox,
	reply_to: Option<Mailbox>,
	subject: &str,
) -> Result<(), Error>
where
	TEmail: EmailTemplate + Send + Sync + 'static,
{
	use lettre::{
		transport::smtp::authentication::Credentials,
		AsyncSmtpTransport,
		AsyncTransport,
		Message,
		Tokio1Executor,
	};
	use tokio::{task, task::JoinHandle};

	use crate::service;

	let subject = subject.to_string();
	let join_handle: JoinHandle<Result<_, Error>> = task::spawn(async move {
		let settings = service::get_config();
		let mut builder = Message::builder()
			.from(settings.email.from.parse()?)
			.to(to.clone())
			.subject(subject);
		if let Some(reply_to) = reply_to {
			builder = builder.reply_to(reply_to);
		}

		let message = builder.multipart(body.render_body().await?)?;

		let credentials = Credentials::new(
			settings.email.username.clone(),
			settings.email.password.clone(),
		);

		let response = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(
			&settings.email.host,
		)?
		.credentials(credentials)
		.port(settings.email.port)
		.build::<Tokio1Executor>()
		.send(message)
		.await?;

		if !response.is_positive() {
			log::error!("Error sending email to `{}`: {}", to, response.code());
		}

		Ok(())
	});

	let _ = task::spawn(async {
		let result = join_handle.await;

		if let Ok(Err(error)) = result {
			// TODO log this error
			log::error!("Unable to send email: {}", error.get_error());
		}
	});

	Ok(())
}

#[cfg(debug_assertions)]
async fn send_email<TEmail>(
	_body: TEmail,
	to: Mailbox,
	_reply_to: Option<Mailbox>,
	_subject: &str,
) -> Result<(), Error>
where
	TEmail: EmailTemplate,
{
	log::trace!("Sending email to {}", to);
	Ok(())
}
