//! Realistic async example: an async function driven once per row.
//! Data file lives in `tests/examples/welcome_emails.yaml`.

use test_data_file::test_data_file;

async fn send_welcome_email(email: &str, _locale: &str) -> bool {
    // stand-in for an async mail enqueue that rejects malformed addresses
    email.contains('@')
}

#[test_data_file(path = "tests/examples/welcome_emails.yaml")]
#[tokio::test]
async fn welcome_email_delivery(email: String, locale: String, expect_sent: bool) {
    assert_eq!(
        send_welcome_email(&email, &locale).await,
        expect_sent,
        "failed for {email}"
    );
}
