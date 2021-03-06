use rendy::{
    command::{QueueId, RenderPassEncoder},
    core::types::vertex::{AsAttribute, AsVertex, VertexFormat},
    factory::{Config, Factory},
    graph::{present::PresentNode, render::*, GraphBuilder, GraphContext, NodeBuffer, NodeImage},
    hal,
    init::AnyWindowedRendy,
    memory::Dynamic,
    resource::{Buffer, BufferInfo, DescriptorSetLayout, Escape, Handle},
    shader::{ShaderKind, SourceLanguage, SourceShaderInfo, SpirvReflection, SpirvShader},
    vulkan::Backend,
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    env_logger::Builder::from_default_env()
        .filter_module("glium_tutorial_but_its_rendy", log::LevelFilter::Trace)
        .init();

    let config: Config = Default::default();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().with_title("glium tutorial but it's rendy");

    let rendy = AnyWindowedRendy::init_auto(&config, window, &event_loop).unwrap();
    rendy::with_any_windowed_rendy!((rendy)
    (mut factory, mut families, surface, window) => {
        let mut graph_builder = GraphBuilder::<Backend, _>::new();

        let size = window.inner_size().to_physical(window.hidpi_factor());

        let color = graph_builder.create_image(
            hal::image::Kind::D2(size.width as u32, size.height as u32, 1, 1),
            1,
            factory.get_surface_format(&surface),
            Some(
                hal::command::ClearValue {
                    color: hal::command::ClearColor{ float32: [0.0, 0.0, 1.0, 1.0] },
                }
            ),
        );

        let pass = graph_builder.add_node(
            TutorialRenderPipeline::builder()
            .into_subpass()
            .with_color(color)
            .into_pass(),
        );

        graph_builder.add_node(PresentNode::builder(&factory, surface, color).with_dependency(pass));

        let mut t: f32 = -0.5;

        let mut graph = graph_builder
        .build(&mut factory, &mut families, &t)
        .unwrap();

        event_loop.run(move |event, _, control_flow| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            Event::EventsCleared => {
                t += 0.0002;
                if t > 0.5 {
                    t = -0.5;
                }

                graph.run(&mut factory, &mut families, &t);
            }
            _ => {}
        });
    });
}

#[derive(Debug, Default)]
struct TutorialRenderPipelineDesc;

impl<B> SimpleGraphicsPipelineDesc<B, f32> for TutorialRenderPipelineDesc
where
    B: hal::Backend,
{
    type Pipeline = TutorialRenderPipeline<B>;

    fn depth_stencil(&self) -> Option<hal::pso::DepthStencilDesc> {
        None
    }

    fn load_shader_set(&self, factory: &mut Factory<B>, _aux: &f32) -> rendy::shader::ShaderSet<B> {
        SHADERS.build(factory, Default::default()).unwrap()
    }

    fn vertices(
        &self,
    ) -> Vec<(
        Vec<hal::pso::Element<hal::format::Format>>,
        hal::pso::ElemStride,
        hal::pso::VertexInputRate,
    )> {
        vec![SHADER_REFLECTION
            .attributes_range(..)
            .unwrap()
            .gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex)]
    }

    fn layout(&self) -> Layout {
        SHADER_REFLECTION.layout().unwrap()
    }

    fn build<'a>(
        self,
        _ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        _queue: QueueId,
        _aux: &f32,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
        set_layouts: &[Handle<DescriptorSetLayout<B>>],
    ) -> Result<TutorialRenderPipeline<B>, hal::pso::CreationError> {
        assert!(buffers.is_empty());
        assert!(images.is_empty());
        assert!(set_layouts.is_empty());

        let vbuf_size = SHADER_REFLECTION.attributes_range(..).unwrap().stride as u64 * 3;

        let mut vbuf = factory
            .create_buffer(
                BufferInfo {
                    size: vbuf_size,
                    usage: hal::buffer::Usage::VERTEX,
                },
                Dynamic,
            )
            .unwrap();

        unsafe {
            factory
                .upload_visible_buffer(
                    &mut vbuf,
                    0,
                    &[
                        Vertex {
                            position: [-0.5, 0.5].into(),
                        },
                        Vertex {
                            position: [0.0, -0.5].into(),
                        },
                        Vertex {
                            position: [0.5, 0.25].into(),
                        },
                    ],
                )
                .unwrap();
        }

        Ok(TutorialRenderPipeline { vertex: vbuf })
    }
}

#[derive(Debug)]
struct TutorialRenderPipeline<B: hal::Backend> {
    vertex: Escape<Buffer<B>>,
}

impl<B> SimpleGraphicsPipeline<B, f32> for TutorialRenderPipeline<B>
where
    B: hal::Backend,
{
    type Desc = TutorialRenderPipelineDesc;

    fn prepare(
        &mut self,
        _factory: &Factory<B>,
        _queue: QueueId,
        _set_layouts: &[Handle<DescriptorSetLayout<B>>],
        _index: usize,
        _aux: &f32,
    ) -> PrepareResult {
        PrepareResult::DrawRecord
    }

    fn draw(
        &mut self,
        layout: &B::PipelineLayout,
        mut encoder: RenderPassEncoder<'_, B>,
        _index: usize,
        aux: &f32,
    ) {
        unsafe {
            encoder.push_constants(
                layout,
                hal::pso::ShaderStageFlags::VERTEX,
                0,
                &[aux.to_bits()],
            );
            encoder.bind_vertex_buffers(0, Some((self.vertex.raw(), 0)));
            encoder.draw(0..3, 0..1);
        }
    }

    fn dispose(self, _factory: &mut Factory<B>, _aux: &f32) {}
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
struct Vertex {
    position: Position,
}

impl AsVertex for Vertex {
    fn vertex() -> VertexFormat {
        VertexFormat::new(Position::vertex())
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Position(pub [f32; 2]);
impl<T> From<T> for Position
where
    T: Into<[f32; 2]>,
{
    fn from(from: T) -> Self {
        Position(from.into())
    }
}
impl AsAttribute for Position {
    const NAME: &'static str = "position";
    const FORMAT: hal::format::Format = hal::format::Format::Rgb32Sfloat;
}

lazy_static::lazy_static! {
    static ref VERTEX: SpirvShader = SourceShaderInfo::new(
        include_str!("03x1.shader.vert"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/03x1.shader.vert").into(),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref FRAGMENT: SpirvShader = SourceShaderInfo::new(
        include_str!("03x1.shader.frag"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/03x1.shader.frag").into(),
        ShaderKind::Fragment,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref SHADERS: rendy::shader::ShaderSetBuilder = rendy::shader::ShaderSetBuilder::default()
        .with_vertex(&*VERTEX).unwrap()
        .with_fragment(&*FRAGMENT).unwrap();
}

lazy_static::lazy_static! {
    static ref SHADER_REFLECTION: SpirvReflection = SHADERS.reflect().unwrap();
}
