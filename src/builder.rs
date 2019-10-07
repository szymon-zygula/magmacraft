custom_error::custom_error!(pub BuilderError
    RequiredFieldNotSpecified = "required field not specified"
);

#[derive(Debug)]
pub struct BuilderRequirement<T> (Option<T>);

impl<T> BuilderRequirement<T> {
    pub fn set(&mut self, owned: T) {
        self.0 = Some(owned)
    }

    pub fn get(&self) -> Result<&T, BuilderError> {
        Ok(self.0.as_ref().ok_or(BuilderError::RequiredFieldNotSpecified)?)
    }

    pub fn get_mut(&mut self) -> Result<&mut T, BuilderError> {
        Ok(self.0.as_mut().ok_or(BuilderError::RequiredFieldNotSpecified)?)
    }

    pub fn take(&mut self) -> Result<T, BuilderError> {
        Ok(self.0.take().ok_or(BuilderError::RequiredFieldNotSpecified)?)
    }
}

impl<T> Default for BuilderRequirement<T> {
    fn default() -> Self { Self(None) }
}

#[derive(Debug)]
pub struct BuilderInternal<T> (Option<T>);

impl<T> BuilderInternal<T> {
    pub fn set(&mut self, owned: T) {
        self.0 = Some(owned)
    }

    pub fn get(&self) -> &T {
        self.0.as_ref().expect("internal builder field not set before use or after")
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.0.as_mut().expect("internal builder field not set before use or after")
    }

    pub fn take(&mut self) -> T {
        self.0.take().expect("internal builder field not set before moving out")
    }
}

impl<T> Default for BuilderInternal<T> {
    fn default() -> Self { Self(None) }
}

#[derive(Debug)]
pub struct BuilderProduct<T> (Option<T>);

impl<T> BuilderProduct<T> {
    pub fn set(&mut self, owned: T) {
        self.0 = Some(owned)
    }

    pub fn unwrap(self) -> T {
        self.0.expect("builder product not created before use")
    }
}

impl<T> Default for BuilderProduct<T> {
    fn default() -> Self { Self(None) }
}
