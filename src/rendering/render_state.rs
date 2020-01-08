use std::{
    collections::HashMap,
    rc::Rc,
    marker::PhantomData
};
use crate::{
    builder::{
        BuilderRequirement,
        BuilderInternal
    },
    rendering::{
        RenderingResult,
        renderer::Renderer
    },
    vulkan::{
        pipeline::{
            Pipeline,
            PipelineBuilder
        },
        shader::ShaderStage
    }
};
pub use crate::vulkan::{
    shader::{
        GeometryShader,
        VertexShader,
        FragmentShader
    },
    command_buffer::PushConstants
};

pub struct RenderState<'a, G, V, F> where
    G: PushConstants + 'a,
    V: PushConstants + 'a,
    F: PushConstants + 'a {
    pipeline: Rc<Pipeline>,
    shaders: HashMap<ShaderStage, Box<dyn PushConstants + 'a>>,
    geometry_constants: PhantomData<G>,
    vertex_constants: PhantomData<V>,
    fragment_constants: PhantomData<F>
}

impl<'a, G, V, F> RenderState<'a, G, V, F> where
    G: PushConstants + 'a,
    V: PushConstants + 'a,
    F: PushConstants + 'a {
    pub fn builder() -> RenderStateBuilder<'a, G, V, F> {
        RenderStateBuilder {
            ..Default::default()
        }
    }

    pub fn pipeline(&self) -> &Rc<Pipeline> {
        &self.pipeline
    }

    pub fn push_geometry_constants(&mut self, constants: G) {
        self.shaders.insert(ShaderStage::Geometry, Box::new(constants));
    }

    pub fn push_vertex_constants(&mut self, constants: V) {
        self.shaders.insert(ShaderStage::Vertex, Box::new(constants));
    }

    pub fn push_fragment_constants(&mut self, constants: F) {
        self.shaders.insert(ShaderStage::Fragment, Box::new(constants));
    }
}

pub trait RenderStateTrait {
    fn pipeline(&self) -> &Rc<Pipeline>;
    fn iterate_shaders(&self)
        -> std::collections::hash_map::IntoIter<ShaderStage, &dyn PushConstants>;
}

impl<'a, G, V, F> RenderStateTrait for RenderState<'a, G, V, F> where
    G: PushConstants + 'a,
    V: PushConstants + 'a,
    F: PushConstants + 'a {
    fn pipeline(&self) -> &Rc<Pipeline> {
        &self.pipeline
    }

    fn iterate_shaders(
        &self
    ) -> std::collections::hash_map::IntoIter<ShaderStage, &dyn PushConstants> {
        self.shaders.iter().map(|(key, value)| {
            (*key, value.as_ref())
        }).collect::<HashMap<ShaderStage, &dyn PushConstants>>().into_iter()
    }
}

pub struct RenderStateBuilder<'a, G, V, F> where
    G: PushConstants,
    V: PushConstants,
    F: PushConstants {
    renderer: BuilderRequirement<&'a Renderer>,
    geometry_shader: Option<&'a GeometryShader>,
    vertex_shader: Option<&'a VertexShader>,
    fragment_shader: Option<&'a FragmentShader>,

    pipeline: BuilderInternal<Pipeline>,

    geometry_constants: PhantomData<G>,
    vertex_constants: PhantomData<V>,
    fragment_constants: PhantomData<F>
}

impl<'a, G, V, F> RenderStateBuilder<'a, G, V, F> where
    G: PushConstants,
    V: PushConstants,
    F: PushConstants {
    pub fn geometry_shader(mut self, shader: &'a GeometryShader) -> Self {
        self.geometry_shader = Some(shader);
        self
    }

    pub fn vertex_shader(mut self, shader: &'a VertexShader) -> Self {
        self.vertex_shader = Some(shader);
        self
    }

    pub fn fragment_shader(mut self, shader: &'a FragmentShader) -> Self {
        self.fragment_shader = Some(shader);
        self
    }

    pub fn renderer(mut self, renderer: &'a Renderer) -> Self {
        self.renderer.set(renderer);
        self
    }

    pub fn build(mut self) -> RenderingResult<RenderState<'static, G, V, F>> {
        self.init_pipeline()?;

        Ok(RenderState {
            pipeline: Rc::new(self.pipeline.take()),
            shaders: HashMap::new(),
            geometry_constants: PhantomData,
            vertex_constants: PhantomData,
            fragment_constants: PhantomData
        })
    }

    fn init_pipeline(&mut self) -> RenderingResult<()> {
        let mut pipeline_builder = Pipeline::builder();
        pipeline_builder = self.add_shaders_to_pipeline_if_some(pipeline_builder);

        let pipeline = pipeline_builder
            .logical_device(Rc::clone(self.renderer.logical_device()))
            .swapchain(Rc::clone(self.renderer.swapchain()))
            .render_pass(Rc::clone(self.renderer.render_pass()))
            .subpass(0)
            .build()?;

        self.pipeline.set(pipeline);

        Ok(())
    }

    fn add_shaders_to_pipeline_if_some(
        &mut self,
        mut pipeline_builder: PipelineBuilder<'a>
    ) -> PipelineBuilder<'a> {
        pipeline_builder =
            Self::add_geometry_shader_to_pipeline_if_some(
                pipeline_builder, self.geometry_shader);

        pipeline_builder =
            Self::add_vertex_shader_to_pipeline_if_some(
                pipeline_builder, self.vertex_shader);

        pipeline_builder =
            Self::add_fragment_shader_to_pipeline_if_some(
                pipeline_builder, self.fragment_shader);

        pipeline_builder
    }

    fn add_geometry_shader_to_pipeline_if_some<'b>(
        pipeline_builder: PipelineBuilder<'b>,
        geometry_shader: Option<&'b GeometryShader>
    ) -> PipelineBuilder<'b> {
        match geometry_shader {
            Some(shader) => pipeline_builder
                .geometry_shader(shader)
                .push_constants_size(ShaderStage::Geometry, std::mem::size_of::<G>()),
            None => pipeline_builder
        }
    }

    fn add_vertex_shader_to_pipeline_if_some<'b>(
        pipeline_builder: PipelineBuilder<'b>,
        vertex_shader: Option<&'b VertexShader>
    ) -> PipelineBuilder<'b> {
        match vertex_shader {
            Some(shader) => pipeline_builder
                .vertex_shader(shader)
                .push_constants_size(ShaderStage::Vertex, std::mem::size_of::<V>()),
            None => pipeline_builder
        }
    }

    fn add_fragment_shader_to_pipeline_if_some<'b>(
        pipeline_builder: PipelineBuilder<'b>,
        fragment_shader: Option<&'b FragmentShader>
    ) -> PipelineBuilder<'b> {
        match fragment_shader {
            Some(shader) => pipeline_builder
                .fragment_shader(shader)
                .push_constants_size(ShaderStage::Fragment, std::mem::size_of::<F>()),
            None => pipeline_builder
        }
    }
}

impl<'a, G, V, F> Default for RenderStateBuilder<'a, G, V, F> where
    G: PushConstants + 'a,
    V: PushConstants + 'a,
    F: PushConstants + 'a {
    fn default() -> Self {
        Self {
            renderer: BuilderRequirement::none(),
            geometry_shader: None,
            vertex_shader: None,
            fragment_shader: None,

            pipeline: BuilderInternal::none(),
            geometry_constants: PhantomData::<G>,
            vertex_constants: PhantomData::<V>,
            fragment_constants: PhantomData::<F>
        }
    }
}
