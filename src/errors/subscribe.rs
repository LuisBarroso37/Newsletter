pub enum SubscribeError {
  ValidationError(String),
  StoreTokenError(StoreTokenError),
  SendEmailError(reqwest::Error),
  ConnectionPoolError(sqlx::Error),
  InsertSubscriberError(sqlx::Error),
  TransactionCommitError(sqlx::Error),
}

impl From<reqwest::Error> for SubscribeError {
  fn from(e: reqwest::Error) -> Self {
      Self::SendEmailError(e)
  }
}

impl From<StoreTokenError> for SubscribeError {
  fn from(e: StoreTokenError) -> Self {
      Self::StoreTokenError(e)
  }
}

impl From<String> for SubscribeError {
  fn from(e: String) -> Self {
      Self::ValidationError(e)
  }
}

impl std::fmt::Debug for SubscribeError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
      error_chain_fmt(self, f)
  }
}

impl std::fmt::Display for SubscribeError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
      match self {
          SubscribeError::ValidationError(e) => write!(f, "{}", e),
          SubscribeError::StoreTokenError(_) => write!(
              f,
              "Failed to store the confirmation token for a new subscriber"
          ),
          SubscribeError::SendEmailError(_) => write!(f, "Failed to send a confirmaton email"),
          SubscribeError::ConnectionPoolError(_) => {
              write!(f, "Failed to acquire a Postgres connection from the pool")
          }
          SubscribeError::InsertSubscriberError(_) => {
              write!(f, "Failed to insert new subscriber in the database")
          }
          SubscribeError::TransactionCommitError(_) => write!(
              f,
              "Failed to commit SQL transaction to store a new subscriber"
          ),
      }
  }
}

impl std::error::Error for SubscribeError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
      match self {
          // &str does not implement `Error` - we consider it the root cause
          SubscribeError::ValidationError(_) => None,
          SubscribeError::SendEmailError(e) => Some(e),
          SubscribeError::StoreTokenError(e) => Some(e),
          SubscribeError::ConnectionPoolError(e) => Some(e),
          SubscribeError::InsertSubscriberError(e) => Some(e),
          SubscribeError::TransactionCommitError(e) => Some(e),
      }
  }
}

impl ResponseError for SubscribeError {
  fn status_code(&self) -> StatusCode {
      match self {
          SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
          SubscribeError::ConnectionPoolError(_)
          | SubscribeError::StoreTokenError(_)
          | SubscribeError::SendEmailError(_)
          | SubscribeError::TransactionCommitError(_)
          | SubscribeError::InsertSubscriberError(_) => StatusCode::INTERNAL_SERVER_ERROR,
      }
  }
}

pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while trying to store a subscription token"
        )
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

fn error_chain_fmt(e: &impl std::error::Error, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    // Top level error
    writeln!(f, "{}\n", e)?;

    // Get source from top level error
    let mut current = e.source();

    // Loop through the whole chain of errors to find the underlying cause
    // of the failure that occurred
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }

    Ok(())
}