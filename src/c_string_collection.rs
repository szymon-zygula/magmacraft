macro_rules! create_c_string_collection_type {
    ($name:ident) => {
        #[derive(Default)]
        pub struct $name {
            // pointers in `pointers` point to memory owned by `strings`
            strings: Vec<std::ffi::CString>,
            pointers: Vec<*const std::os::raw::c_char>
        }

        impl $name {
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

            pub fn new() -> Self {
                Self {
                    strings: Vec::new(),
                    pointers: Vec::new()
                }
            }

            pub fn push(&mut self, string: &str) {
                self.add_string(string);
                self.add_pointer_to_last_c_string();
            }

            fn add_string(&mut self, string: &str) {
                // str is guaranteed to be valid UTF-8, so it is impossible for `CString::new` to
                // return an error
                let c_string = std::ffi::CString::new(string.as_bytes()).unwrap();
                self.strings.push(c_string);
            }

            fn add_pointer_to_last_c_string(&mut self) {
                let pointer = self.strings.last().unwrap().as_ptr() as *const std::os::raw::c_char;
                self.pointers.push(pointer);
            }

            pub fn get_pointers(&self) -> &[*const std::os::raw::c_char] {
                self.pointers.as_slice()
            }

            pub fn get_strings(&self) -> &Vec<std::ffi::CString> {
                &self.strings
            }

            pub fn len(&self) -> usize {
                self.strings.len()
            }
        }
    }
}

create_c_string_collection_type!(CStringCollection);
