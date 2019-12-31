macro_rules! declare_builder_field {
    ($name:ident) => {
        #[derive(Debug)]
        pub struct $name<T> (Option<T>);

        impl<T> $name<T> {
            const ERROR_MESSAGE: &'static str =
                "field not set or taken before use";

            pub fn none() -> Self {
                Self(None)
            }

            pub fn set(&mut self, owned: T) {
                self.0 = Some(owned);
            }

            pub fn take(&mut self) -> T {
                let error_message = format!("{} {}", Self::ERROR_MESSAGE, stringify!($name));
                self.0.take().expect(&error_message)
            }
        }

        impl<T> std::ops::Deref for $name<T> {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                self.as_ref()
            }
        }

        impl<T> std::ops::DerefMut for $name<T> {
            fn deref_mut(&mut self) -> &mut T {
                self.as_mut()
            }
        }

        impl<T> AsRef<T> for $name<T> {
            fn as_ref(&self) -> &T {
                let error_message = format!("{} {}", Self::ERROR_MESSAGE, stringify!($name));
                self.0.as_ref().expect(&error_message)
            }
        }

        impl<T> AsMut<T> for $name<T> {
            fn as_mut(&mut self) -> &mut T {
                let error_message = format!("{} {}", Self::ERROR_MESSAGE, stringify!($name));
                self.0.as_mut().expect(&error_message)
            }
        }

        impl<T> Default for $name<T> {
            fn default() -> Self {
                Self(None)
            }
        }
    }
}

declare_builder_field!(BuilderInternal);
declare_builder_field!(BuilderRequirement);

#[derive(Debug)]
pub struct BuilderProduct<T> (Option<T>);

impl<T> BuilderProduct<T> {
    pub fn none() -> Self {
        Self(None)
    }

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
