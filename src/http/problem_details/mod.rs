pub(crate) use self::problem_type::{ProblemDescription, ProblemType};

mod problem_type;

pub(crate) type ProblemDetails = ::problem_details::ProblemDetails<ProblemType>;
