/// Listening socket file number
pub const FCGI_LISTENSOCK_FILENO: u32 = 0;



/// Number of bytes in a FCGI_Header.  Future versions of the protocol
/// will not reduce this number.
pub const FCGI_HEADER_LEN: u16 = 8;


/// Value for version component of FCGI_Header
pub const FCGI_VERSION_1:u8 = 1;

/// Values for type component of FCGI_Header

pub const FCGI_BEGIN_REQUEST: u8 = 1;
pub const FCGI_ABORT_REQUEST: u8 = 2;
pub const FCGI_END_REQUEST: u8 = 3;
pub const FCGI_PARAMS: u8 = 4;
pub const FCGI_STDIN: u8 = 5;
pub const FCGI_STDOUT: u8 = 6;
pub const FCGI_STDERR: u8 = 7;
pub const FCGI_DATA: u8 = 8;
pub const FCGI_GET_VALUES: u8 = 9;
pub const FCGI_GET_VALUES_RESULT: u8 = 10;
pub const FCGI_UNKNOWN_TYPE: u8 = 11;

/// Value for requestId component of FCGI_Header
pub const FCGI_NULL_REQUEST_ID: u16 = 0;

/// Mask for flags component of FCGI_BeginRequestBody
pub const FCGI_KEEP_CONN: u8 = 1;

/// Values for role component of FCGI_BeginRequestBody
pub const FCGI_RESPONDER: u16 = 1;
pub const FCGI_AUTHORIZER: u16 = 2;
pub const FCGI_FILTER: u16 = 3;

/// Values for protocolStatus component of FCGI_EndRequestBody
pub const FCGI_REQUEST_COMPLETE: u8 = 0;
pub const FCGI_CANT_MPX_CONN: u8 = 1;
pub const FCGI_OVERLOADED: u8 = 2;
pub const FCGI_UNKNOWN_ROLE: u8 = 3;

/// Variable names for FCGI_GET_VALUES / FCGI_GET_VALUES_RESULT records
pub const FCGI_MAX_CONNS: &'static str = "FCGI_MAX_CONNS";
pub const FCGI_MAX_REQS: &'static str = "FCGI_MAX_REQS";
pub const FCGI_MPXS_CONNS: &'static str = "FCGI_MPXS_CONNS";


