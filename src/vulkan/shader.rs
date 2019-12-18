use std::{
    convert::TryInto,
    fs,
    mem,
    rc::Rc,
    io::Read
};
use ash::{
    self,
    vk,
    version::DeviceV1_0
};
use crate::{
    double_type_buffer::DoubleTypeBuffer,
    vulkan::{
        VulkanResult,
        VulkanError,
        logical_device::LogicalDevice
    }
};

macro_rules! create_shader_wrapper {
    ($name:ident, $shader_stage:expr) => {
        pub struct $name (Shader);

        impl $name {
            pub fn from_file(
                logical_device: Rc<LogicalDevice>,
                file_path: &std::path::Path
            ) -> VulkanResult<Self> {
                let shader = Shader::from_file(file_path, logical_device, $shader_stage);

                match shader {
                    Err(e) => Err(e),
                    Ok(shader) => Ok($name (shader))
                }
            }
        }

        impl ShaderStageBuilder for $name {
            fn shader_stage_create_info_builder(&self) -> vk::PipelineShaderStageCreateInfoBuilder {
                self.0.shader_stage_create_info_builder()
            }
        }

        impl std::ops::Deref for $name {
            type Target = Shader;

            fn deref(&self) -> &Self::Target {
                self.as_ref()
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Shader {
                self.as_mut()
            }
        }

        impl AsRef<Shader> for $name {
            fn as_ref(&self) -> &Shader {
                &self.0
            }
        }

        impl AsMut<Shader> for $name {
            fn as_mut(&mut self) -> &mut Shader {
                &mut self.0
            }
        }
    }
}

create_shader_wrapper!(VertexShader, vk::ShaderStageFlags::VERTEX);
create_shader_wrapper!(FragmentShader, vk::ShaderStageFlags::FRAGMENT);
create_shader_wrapper!(GeometryShader, vk::ShaderStageFlags::GEOMETRY);

pub struct Shader {
    vk_shader_module: vk::ShaderModule,
    logical_device: Rc<LogicalDevice>,
    shader_stage: vk::ShaderStageFlags
}

impl Shader {
    const SHADER_STAGE_ENTRY_POINT_NAME: &'static [u8] = "main\0".as_bytes();

    fn from_file(
        file_path: &std::path::Path,
        logical_device: Rc<LogicalDevice>,
        shader_stage: vk::ShaderStageFlags
    ) -> VulkanResult<Self> {
        let buffer = Self::load_file_to_buffer(file_path)?;
        let vk_shader_module = Self::create_shader_module(&logical_device, &buffer)?;

        Ok(Self {
            logical_device: Rc::clone(&logical_device),
            vk_shader_module,
            shader_stage
        })
    }

    fn load_file_to_buffer(
        file_path: &std::path::Path
    ) -> VulkanResult<DoubleTypeBuffer<u8, u32>> {
        let mut file = fs::File::open(file_path)
            .map_err(|error| VulkanError::ShaderOpenFileError {error})?;

        let mut buffer = Self::create_buffer_for_file(&file)?;
        Self::read_file_to_buffer(&mut buffer, &mut file);

        Ok(buffer)
    }

    fn create_buffer_for_file(file: &fs::File) -> VulkanResult<DoubleTypeBuffer<u8, u32>> {
        let metadata = file.metadata()
            .map_err(|error| VulkanError::ShaderOpenFileError {error})?;

        let buffer_size = metadata.len().try_into().unwrap();
        let u32_slice_length =
            (buffer_size + mem::size_of::<u32>() - 1) / mem::size_of::<u32>();

        Ok(DoubleTypeBuffer::with_lengths::<u8, u32>(buffer_size, u32_slice_length))
    }

    fn read_file_to_buffer(buffer: &mut DoubleTypeBuffer<u8, u32>, file: &mut fs::File) {
        let buffer_slice_u8 = buffer.as_mut_slice_first();
        let mut eof_reached = false;
        let mut current_byte = 0;
        while !eof_reached {
            let bytes_read = file.read(&mut buffer_slice_u8[current_byte..]).unwrap();
            current_byte += bytes_read;
            eof_reached = bytes_read == 0;
        }
    }

    fn create_shader_module(
        logical_device: &LogicalDevice, code: &DoubleTypeBuffer<u8, u32>
    ) -> VulkanResult<vk::ShaderModule> {
        let buffer_slice_u32 = code.as_slice_second();
        let builder = vk::ShaderModuleCreateInfo::builder()
            .code(buffer_slice_u32);

        Ok(unsafe {
            logical_device.create_shader_module(&builder, None)
                .map_err(|result| {
                    VulkanError::ShaderCreateError {result}
                })?
        })
    }

    pub fn shader_stage_create_info_builder(&self) -> vk::PipelineShaderStageCreateInfoBuilder {
        let entry_point_name =
            std::ffi::CStr::from_bytes_with_nul(Self::SHADER_STAGE_ENTRY_POINT_NAME).unwrap();

        vk::PipelineShaderStageCreateInfo::builder()
            .module(self.vk_shader_module)
            .name(entry_point_name)
            .stage(self.shader_stage)
    }
}

impl ShaderStageBuilder for Shader {
    fn shader_stage_create_info_builder(&self) -> vk::PipelineShaderStageCreateInfoBuilder {
        self.shader_stage_create_info_builder()
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            self.logical_device.destroy_shader_module(self.vk_shader_module, None);
        }
    }
}

pub trait ShaderStageBuilder {
    fn shader_stage_create_info_builder(&self) -> vk::PipelineShaderStageCreateInfoBuilder;
}
