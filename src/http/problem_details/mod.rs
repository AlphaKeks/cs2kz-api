mod problem_type;

pub(crate) use self::problem_type::{ProblemDescription, ProblemType};

pub(crate) type ProblemDetails = ::problem_details::ProblemDetails<ProblemType>;
