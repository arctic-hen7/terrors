use crate::{
    type_set::{Narrow, TupleForm, TypeSet},
    OneOf,
};

pub trait OneOfResult<T, E: TypeSet> {
    fn narrow_err<Target, Index>(
        self,
    ) -> Result<
        Result<T, Target>,
        OneOf<<<E::Variants as Narrow<Target, Index>>::Remainder as TupleForm>::Tuple>,
    >
    where
        Target: 'static,
        E::Variants: Narrow<Target, Index>;
}
impl<T, E: TypeSet> OneOfResult<T, E> for Result<T, OneOf<E>> {
    fn narrow_err<Target, Index>(
        self,
    ) -> Result<
        Result<T, Target>,
        OneOf<<<<E as TypeSet>::Variants as Narrow<Target, Index>>::Remainder as TupleForm>::Tuple>,
    >
    where
        Target: 'static,
        <E as TypeSet>::Variants: Narrow<Target, Index>,
    {
        match self {
            Ok(t) => Ok(Ok(t)),
            Err(err) => {
                let err = err.narrow::<Target, Index>()?;
                Ok(Err(err))
            }
        }
    }
}
