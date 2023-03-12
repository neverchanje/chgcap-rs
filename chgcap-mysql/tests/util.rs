/// Util macro to generate error when the two arguments are not equal.
#[macro_export]
macro_rules! ensure_eq {
    ($left:expr, $right:expr) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(left_val == right_val) {
                    anyhow::bail!(
                        "{} == {} assertion failed.\n`{}` is {}\n`{}` is {}",
                        stringify!($left),
                        stringify!($right),
                        stringify!($left),
                        &*left_val,
                        stringify!($right),
                        &*right_val,
                    );
                }
            }
        }
    };
}
