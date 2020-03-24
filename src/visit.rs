use crate::io::DecodeError;

pub enum VisitError<E> {
    LazyDecode(DecodeError),
    Custom(E),
}

impl<E: std::fmt::Display> std::fmt::Display for VisitError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VisitError::LazyDecode(err) => err.fmt(f),
            VisitError::Custom(err) => err.fmt(f),
        }
    }
}

impl<E: std::fmt::Debug> std::fmt::Debug for VisitError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VisitError::LazyDecode(err) => err.fmt(f),
            VisitError::Custom(err) => err.fmt(f),
        }
    }
}

impl<E: std::error::Error> std::error::Error for VisitError<E> {}

#[cfg(feature = "nightly")]
pub type NeverError = !;

#[cfg(not(feature = "nightly"))]
pub enum NeverError {}

impl From<VisitError<NeverError>> for DecodeError {
    fn from(err: VisitError<NeverError>) -> Self {
        match err {
            VisitError::Custom(err) => match err {},
            VisitError::LazyDecode(err) => err,
        }
    }
}

pub trait VisitResult {
    type Error;

    fn as_result(self) -> Result<(), Self::Error>;
}

impl VisitResult for () {
    type Error = NeverError;

    fn as_result(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl VisitResult for bool {
    type Error = ();

    fn as_result(self) -> Result<(), Self::Error> {
        match self {
            true => Ok(()),
            false => Err(()),
        }
    }
}

impl<E> VisitResult for Result<(), E> {
    type Error = E;

    fn as_result(self) -> Result<(), Self::Error> {
        self
    }
}

pub use wasmbin_derive::WasmbinVisit;
pub trait WasmbinVisit: 'static + Sized {
    fn visit<'a, T: 'static, R: VisitResult, F: FnMut(&'a T) -> R>(
        &'a self,
        mut f: F,
    ) -> Result<(), VisitError<R::Error>> {
        self.visit_child(&mut move |item| f(item).as_result())
    }

    fn visit_mut<'a, T: 'static, R: VisitResult, F: FnMut(&'a mut T) -> R>(
        &'a mut self,
        mut f: F,
    ) -> Result<(), VisitError<R::Error>> {
        self.visit_child_mut(&mut move |item| f(item).as_result())
    }

    fn visit_child<'a, T: 'static, E, F: FnMut(&'a T) -> Result<(), E>>(
        &'a self,
        f: &mut F,
    ) -> Result<(), VisitError<E>> {
        match std::any::Any::downcast_ref(self) {
            Some(v) => f(v).map_err(VisitError::Custom),
            None => self.visit_children(f),
        }
    }

    fn visit_child_mut<'a, T: 'static, E, F: FnMut(&'a mut T) -> Result<(), E>>(
        &'a mut self,
        f: &mut F,
    ) -> Result<(), VisitError<E>> {
        match std::any::Any::downcast_mut(self) {
            Some(v) => f(unsafe {
                // Working around an apparent bug in NLL: https://github.com/rust-lang/rust/issues/70255
                &mut *(v as *mut _)
            })
            .map_err(VisitError::Custom),
            None => self.visit_children_mut(f),
        }
    }

    fn visit_children<'a, T: 'static, E, F: FnMut(&'a T) -> Result<(), E>>(
        &'a self,
        _f: &mut F,
    ) -> Result<(), VisitError<E>> {
        Ok(())
    }

    fn visit_children_mut<'a, T: 'static, E, F: FnMut(&'a mut T) -> Result<(), E>>(
        &'a mut self,
        _f: &mut F,
    ) -> Result<(), VisitError<E>> {
        Ok(())
    }
}