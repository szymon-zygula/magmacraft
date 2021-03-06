macro_rules! create_c_string_collection_type {
    ($name:ident) => {
        #[derive(Default)]
        pub struct $name {
            // Pointers in `pointers` point to memory owned by `strings`.
            // `CString`s owned by `strings` are never reallocated, so `pointers` are always valid
            strings: Vec<std::ffi::CString>,
            pointers: Vec<*const std::os::raw::c_char>
        }

        impl $name {
            pub fn new() -> Self {
                Self {
                    strings: Vec::new(),
                    pointers: Vec::new()
                }
            }

            pub fn from_vec(strings: Vec<String>) -> Self {
                let vec_size = strings.len();
                let mut c_strings = Self::with_capacity(vec_size);

                for string in strings.into_iter() {
                    c_strings.push(&string);
                }

                c_strings
            }

            pub fn with_capacity(capacity: usize) -> Self {
                Self {
                    strings: Vec::with_capacity(capacity),
                    pointers: Vec::with_capacity(capacity)
                }
            }

            pub fn push(&mut self, string: &str) {
                self.add_string(string);
                self.add_next_c_string_pointer();
            }

            fn add_string(&mut self, string: &str) {
                // str is guaranteed to be valid UTF-8, so it is impossible for `CString::new` to
                // return an error
                let c_string = std::ffi::CString::new(string.as_bytes()).unwrap();
                self.strings.push(c_string);
            }

            fn reconstruct_pointers(&mut self) {
                let len = self.strings.len();
                self.pointers = Vec::with_capacity(len);
                for _ in 0..len {
                    self.add_next_c_string_pointer();
                }
            }

            fn add_next_c_string_pointer(&mut self) {
                let index = self.pointers.len();
                let pointer = self.strings[index].as_ptr() as *const std::os::raw::c_char;
                self.pointers.push(pointer);
            }

            pub fn pointers(&self) -> &[*const std::os::raw::c_char] {
                self.pointers.as_slice()
            }

            pub fn strings(&self) -> &Vec<std::ffi::CString> {
                &self.strings
            }

            pub fn len(&self) -> usize {
                self.strings.len()
            }
        }

        impl Clone for $name {
            fn clone(&self) -> Self {
                let mut cloned = Self {
                    strings: self.strings.clone(),
                    pointers: Vec::with_capacity(self.len())
                };

                cloned.reconstruct_pointers();

                cloned
            }
        }
    }
}

create_c_string_collection_type!(CStringCollection);

macro_rules! c_string_collection {
    ($collection:ident: [$($item:expr),+]) => {
        {
            let mut collection = $collection::new();
            $(
                collection.push($item);
            )*
            collection
        }
    }
}
