use ash::vk;
use shaderc::{CompileOptions, Compiler, ShaderKind};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShaderError {
    #[error("Failed to create shaderc compiler")]
    CompilerInit,
    #[error("Compilation error: {0}")]
    Compile(#[from] shaderc::Error),
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
    #[error("SPIR-V size is not a multiple of 4")]
    InvalidSpirv,
}

pub struct VulkanShader {
    pub module: vk::ShaderModule,
    device:     ash::Device,
}

impl VulkanShader {
    pub fn from_glsl(
        device:      &ash::Device,
        source:      &str,
        kind:        ShaderKind,
        entry_point: &str,
    ) -> Result<Self, ShaderError> {
        let compiler = Compiler::new()?;
        let mut options = CompileOptions::new()?;

        options.set_target_env(
            shaderc::TargetEnv::Vulkan,
            shaderc::EnvVersion::Vulkan1_3 as u32,
        );
        options.set_optimization_level(shaderc::OptimizationLevel::Performance);

        let artifact = compiler.compile_into_spirv(
            source,
            kind,
            "shader",
            entry_point,
            Some(&options),
        )?;

        let spirv = artifact.as_binary_u8();
        if spirv.len() % 4 != 0 {
            return Err(ShaderError::InvalidSpirv);
        }

        // SAFETY: SPIR-V is validated to be 4-byte aligned above
        let code = unsafe {
            std::slice::from_raw_parts(spirv.as_ptr() as *const u32, spirv.len() / 4)
        };

        let create_info = vk::ShaderModuleCreateInfo::default().code(code);
        let module      = unsafe { device.create_shader_module(&create_info, None)? };

        Ok(Self { module, device: device.clone() })
    }
}

impl Drop for VulkanShader {
    fn drop(&mut self) {
        unsafe { self.device.destroy_shader_module(self.module, None) };
    }
}

// ---------------------------------------------------------------------------
// Shader sources
// ---------------------------------------------------------------------------

pub const VERTEX_SHADER: &str = r#"
#version 450

layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
} ubo;

layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_color;

layout(location = 0) out vec3 frag_color;

void main() {
    gl_Position = ubo.proj * ubo.view * ubo.model * vec4(in_position, 1.0);
    frag_color  = in_color;
}
"#;

pub const FRAGMENT_SHADER: &str = r#"
#version 450

layout(location = 0) in  vec3 frag_color;
layout(location = 0) out vec4 out_color;

void main() {
    out_color = vec4(frag_color, 1.0);
}
"#;