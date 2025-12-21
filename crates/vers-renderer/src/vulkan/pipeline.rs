use ash::vk;
use shaderc::ShaderKind;
use thiserror::Error;

use super::device::VulkanDevice;
use super::render_pass::VulkanRenderPass;
use super::shader::{ShaderError, VulkanShader, FRAGMENT_SHADER, VERTEX_SHADER};

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("Shader error: {0}")]
    Shader(#[from] ShaderError),
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
}

pub struct VulkanPipeline {
    pub pipeline:        vk::Pipeline,
    pub layout:          vk::PipelineLayout,
    pub descriptor_layout: vk::DescriptorSetLayout,
    device:              ash::Device,
}

impl VulkanPipeline {
    pub fn new(
        device:      &VulkanDevice,
        render_pass: &VulkanRenderPass,
    ) -> Result<Self, PipelineError> {
        // --- Descriptor set layout (UBO at binding 0, vertex stage) ---------
        let ubo_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX);

        let bindings = [ubo_binding];
        let descriptor_layout = unsafe {
            device.device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings),
                None,
            )?
        };

        // --- Pipeline layout ------------------------------------------------
        let set_layouts = [descriptor_layout];
        let layout = unsafe {
            device.device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts),
                None,
            )?
        };

        // --- Shaders --------------------------------------------------------
        let vert = VulkanShader::from_glsl(&device.device, VERTEX_SHADER,   ShaderKind::Vertex,   "main")?;
        let frag = VulkanShader::from_glsl(&device.device, FRAGMENT_SHADER, ShaderKind::Fragment, "main")?;

        let entry = std::ffi::CString::new("main").unwrap();

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert.module)
                .name(&entry),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag.module)
                .name(&entry),
        ];

        // --- Vertex input ---------------------------------------------------
        // Vertex: vec3 position + vec3 color = 24 bytes
        let binding_desc = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX);

        let attr_descs = Vertex::attribute_descriptions();

        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(std::slice::from_ref(&binding_desc))
            .vertex_attribute_descriptions(&attr_descs);

        // --- Fixed-function state -------------------------------------------
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false);

        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false);

        let blend_attachments = [color_blend_attachment];
        let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(&blend_attachments);

        // Viewport and scissor are dynamic — set per frame, not baked into pipeline
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&dynamic_states);

        // --- Create pipeline ------------------------------------------------
        let create_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .depth_stencil_state(&depth_stencil)
            .color_blend_state(&color_blending)
            .dynamic_state(&dynamic_state)
            .layout(layout)
            .render_pass(render_pass.render_pass)
            .subpass(0);

        let pipeline = unsafe {
            device.device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
                .map_err(|(_, e)| e)?[0]
        };

        Ok(Self { pipeline, layout, descriptor_layout, device: device.device.clone() })
    }
}

impl Drop for VulkanPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.layout, None);
            self.device.destroy_descriptor_set_layout(self.descriptor_layout, None);
        }
    }
}

// ---------------------------------------------------------------------------
// Vertex type
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color:    [f32; 3],
}

impl Vertex {
    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        [
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(std::mem::size_of::<[f32; 3]>() as u32),
        ]
    }
}