use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct WrapperError
{
    prefix: String,
    wrapped: Box<dyn Error + Send + 'static>
}

impl WrapperError
{
    pub fn new(prefix: &str, err: Box<dyn Error + Send + 'static>)
               -> WrapperError
    {
        WrapperError{
            prefix: prefix.to_string(),
            wrapped: err
        }
    }
}

impl Error for WrapperError
{
    fn source(&self) -> Option<&(dyn Error + 'static)>
    {
        Some(self.wrapped.as_ref())
    }
}

impl fmt::Display for WrapperError
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f, "{}: {}", self.prefix, self.wrapped)
    }
}
