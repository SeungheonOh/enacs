use std::sync::Arc;

use glyphon::{
    Attrs, Buffer as GlyphonBuffer, Cache, Color as GlyphonColor, Family, FontSystem, Metrics,
    Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};

use crate::syntax::{Highlight, Theme as SyntaxTheme};
use wgpu::{
    include_wgsl, util::DeviceExt, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType, BufferUsages,
    CommandEncoderDescriptor, CompositeAlphaMode, DeviceDescriptor, Features, FragmentState,
    Instance, InstanceDescriptor, Limits, LoadOp, MultisampleState, Operations,
    PipelineLayoutDescriptor, PresentMode, PrimitiveState, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions,
    ShaderStages, StoreOp, Surface, SurfaceConfiguration, TextureFormat, TextureUsages,
    TextureViewDescriptor, VertexState,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, KeyEvent as WinitKeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{Window, WindowAttributes, WindowId},
};

use crate::keybinding::key::Modifiers;
use crate::keybinding::KeyEvent;
use crate::state::EditorState;

use super::traits::{Frontend, FrontendCapabilities, FrontendError};

const FONT_SIZE: f32 = 28.0;
const CELL_HEIGHT: f32 = FONT_SIZE;
const FONT_FAMILY: &str = "Comic Mono";

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Theme {
    background: [f32; 4],
    foreground: GlyphonColor,
    cursor_bg: [f32; 4],
    cursor_fg: GlyphonColor,
    selection: [f32; 4],
    modeline_bg: [f32; 4],
    modeline_fg: GlyphonColor,
}

fn hex_to_rgba(hex: u32) -> [f32; 4] {
    let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
    let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
    let b = (hex & 0xFF) as f32 / 255.0;
    [r, g, b, 1.0]
}

fn hex_to_rgba_alpha(hex: u32, alpha: f32) -> [f32; 4] {
    let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
    let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
    let b = (hex & 0xFF) as f32 / 255.0;
    [r, g, b, alpha]
}

fn hex_to_color(hex: u32) -> GlyphonColor {
    let r = ((hex >> 16) & 0xFF) as u8;
    let g = ((hex >> 8) & 0xFF) as u8;
    let b = (hex & 0xFF) as u8;
    GlyphonColor::rgb(r, g, b)
}

fn expand_tabs(s: &str, tab_width: usize) -> String {
    let mut result = String::with_capacity(s.len());
    let mut col = 0;
    for ch in s.chars() {
        if ch == '\t' {
            let spaces = tab_width - (col % tab_width);
            for _ in 0..spaces {
                result.push(' ');
            }
            col += spaces;
        } else {
            result.push(ch);
            col += 1;
        }
    }
    result
}

const TAB_WIDTH: usize = 4;

fn is_shifted_symbol(ch: char) -> bool {
    // Characters that require Shift on a US keyboard layout
    matches!(
        ch,
        '!' | '@'
            | '#'
            | '$'
            | '%'
            | '^'
            | '&'
            | '*'
            | '('
            | ')'
            | '_'
            | '+'
            | '{'
            | '}'
            | '|'
            | ':'
            | '"'
            | '<'
            | '>'
            | '?'
            | '~'
    )
}

fn char_col_to_visual_col(line: &str, char_col: usize) -> usize {
    let mut visual_col = 0;
    for (i, ch) in line.chars().enumerate() {
        if i >= char_col {
            break;
        }
        if ch == '\t' {
            visual_col += TAB_WIDTH - (visual_col % TAB_WIDTH);
        } else {
            visual_col += 1;
        }
    }
    visual_col
}

fn find_highlight_at_byte(highlights: &[Highlight], byte_offset: usize) -> Option<&Highlight> {
    highlights
        .iter()
        .find(|h| byte_offset >= h.byte_range.start && byte_offset < h.byte_range.end)
}

fn build_highlighted_spans(
    text: &str,
    highlights: &[Highlight],
    line_start_byte: usize,
    syntax_theme: &SyntaxTheme,
    default_color: GlyphonColor,
    spans: &mut Vec<(String, GlyphonColor)>,
) {
    if text.is_empty() {
        return;
    }

    let mut current_span = String::new();
    let mut current_color = default_color;
    let mut byte_offset = line_start_byte;
    let mut visual_col = 0;

    for ch in text.chars() {
        let color = find_highlight_at_byte(highlights, byte_offset)
            .map(|h| syntax_theme.color_for(h.style).to_glyphon())
            .unwrap_or(default_color);

        if color != current_color && !current_span.is_empty() {
            spans.push((std::mem::take(&mut current_span), current_color));
        }
        current_color = color;

        if ch == '\t' {
            let spaces = TAB_WIDTH - (visual_col % TAB_WIDTH);
            for _ in 0..spaces {
                current_span.push(' ');
            }
            visual_col += spaces;
        } else {
            current_span.push(ch);
            visual_col += 1;
        }

        byte_offset += ch.len_utf8();
    }

    if !current_span.is_empty() {
        spans.push((current_span, current_color));
    }
}

impl Default for Theme {
    fn default() -> Self {
        // Modus Operandi - light theme
        Self {
            background: hex_to_rgba(0xffffff),
            foreground: hex_to_color(0x000000),
            cursor_bg: hex_to_rgba(0x000000),
            cursor_fg: hex_to_color(0xffffff),
            selection: hex_to_rgba_alpha(0xbdbdbd, 0.8),
            modeline_bg: hex_to_rgba(0xc4c4c4),
            modeline_fg: hex_to_color(0x000000),
        }
    }
}

pub struct GuiFrontend {
    initialized: bool,
}

impl GuiFrontend {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

impl Default for GuiFrontend {
    fn default() -> Self {
        Self::new()
    }
}

impl Frontend for GuiFrontend {
    fn init(&mut self) -> Result<(), FrontendError> {
        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), FrontendError> {
        Ok(())
    }

    fn size(&self) -> (u16, u16) {
        (120, 40)
    }

    fn run(self, state: EditorState) -> Result<(), FrontendError> {
        let event_loop = EventLoop::new().map_err(|e| FrontendError::Gui(e.to_string()))?;
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut app = GuiApp::new(state);

        event_loop
            .run_app(&mut app)
            .map_err(|e| FrontendError::Gui(e.to_string()))?;

        Ok(())
    }

    fn render(&mut self, _state: &EditorState) -> Result<(), FrontendError> {
        Ok(())
    }

    fn bell(&mut self) {}

    fn capabilities(&self) -> FrontendCapabilities {
        FrontendCapabilities {
            images: false,
            true_color: true,
            clipboard: true,
            variable_width_fonts: true,
        }
    }

    fn pixel_size(&self) -> Option<(u32, u32)> {
        Some((1200, 800))
    }
}

struct GpuState {
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    rect_pipeline: RenderPipeline,
    rect_bind_group_layout: BindGroupLayout,
}

struct TextState {
    font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct RectUniforms {
    rect: [f32; 4],
    color: [f32; 4],
    screen_size: [f32; 2],
    _padding: [f32; 2],
}

struct GuiApp {
    state: EditorState,
    window: Option<Arc<Window>>,
    gpu: Option<GpuState>,
    text: Option<TextState>,
    theme: Theme,
    modifiers: ModifiersState,
    cols: u16,
    rows: u16,
    cell_width: f32,
    cell_height: f32,
}

impl GuiApp {
    fn new(state: EditorState) -> Self {
        Self {
            state,
            window: None,
            gpu: None,
            text: None,
            theme: Theme::default(),
            modifiers: ModifiersState::empty(),
            cols: 80,
            rows: 24,
            cell_width: FONT_SIZE * 0.6, // Placeholder, will be measured
            cell_height: CELL_HEIGHT,
        }
    }

    fn grid_to_pixel(&self, col: u16, row: u16) -> (f32, f32) {
        (col as f32 * self.cell_width, row as f32 * self.cell_height)
    }

    fn init_gpu(&mut self, window: Arc<Window>) {
        let size = window.inner_size();

        let instance = Instance::new(InstanceDescriptor::default());

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("Failed to find adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &DeviceDescriptor {
                label: None,
                required_features: Features::empty(),
                required_limits:
                    Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
                memory_hints: Default::default(),
            },
            None,
        ))
        .expect("Failed to create device");

        let swapchain_format = TextureFormat::Bgra8UnormSrgb;

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let rect_shader = device.create_shader_module(include_wgsl!("rect.wgsl"));

        let rect_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Rect Bind Group Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let rect_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Rect Pipeline Layout"),
            bind_group_layouts: &[&rect_bind_group_layout],
            push_constant_ranges: &[],
        });

        let rect_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Rect Pipeline"),
            layout: Some(&rect_pipeline_layout),
            vertex: VertexState {
                module: &rect_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &rect_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: swapchain_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        self.gpu = Some(GpuState {
            surface,
            config,
            device,
            queue,
            rect_pipeline,
            rect_bind_group_layout,
        });

        self.init_text();
    }

    fn init_text(&mut self) {
        let gpu = self.gpu.as_ref().unwrap();

        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(&gpu.device);
        let mut atlas = TextAtlas::new(&gpu.device, &gpu.queue, &cache, gpu.config.format);
        let text_renderer =
            TextRenderer::new(&mut atlas, &gpu.device, MultisampleState::default(), None);

        let viewport = Viewport::new(&gpu.device, &cache);

        // Measure actual character width from the font
        self.cell_width = Self::measure_char_width(&mut font_system);

        self.text = Some(TextState {
            font_system,
            swash_cache,
            atlas,
            text_renderer,
            viewport,
        });
    }

    fn measure_char_width(font_system: &mut FontSystem) -> f32 {
        let metrics = Metrics::new(FONT_SIZE, CELL_HEIGHT);
        let mut buffer = GlyphonBuffer::new(font_system, metrics);
        buffer.set_size(font_system, Some(1000.0), Some(CELL_HEIGHT));
        buffer.set_text(
            font_system,
            "M",
            Attrs::new().family(Family::Name(FONT_FAMILY)),
            Shaping::Advanced,
        );

        // Get the width of the character from the layout
        let layout = buffer.layout_runs().next();
        if let Some(run) = layout {
            if let Some(glyph) = run.glyphs.first() {
                return glyph.w;
            }
        }

        // Fallback if measurement fails
        FONT_SIZE * 0.6
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        if let Some(gpu) = &mut self.gpu {
            gpu.config.width = size.width;
            gpu.config.height = size.height;
            gpu.surface.configure(&gpu.device, &gpu.config);
        }

        self.cols = (size.width as f32 / self.cell_width) as u16;
        self.rows = (size.height as f32 / self.cell_height) as u16;

        // Content area is rows - 2 (modeline at row-2, minibuffer at row-1)
        self.state
            .set_dimensions(self.cols, self.rows.saturating_sub(2));
    }

    fn create_rect_bind_group(gpu: &GpuState, uniforms: RectUniforms) -> BindGroup {
        let buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Rect Uniform Buffer"),
                contents: bytemuck::cast_slice(&[uniforms]),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });

        gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Rect Bind Group"),
            layout: &gpu.rect_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        })
    }

    fn render(&mut self) {
        self.state.update_visible_highlights();

        let gpu = match &self.gpu {
            Some(g) => g,
            None => return,
        };

        let frame = match gpu.surface.get_current_texture() {
            Ok(f) => f,
            Err(_) => return,
        };

        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        let pixel_width = gpu.config.width as f32;
        let pixel_height = gpu.config.height as f32;
        let gpu_width = gpu.config.width;
        let gpu_height = gpu.config.height;

        let theme = self.theme;

        // Grid layout (like terminal):
        // - rows 0 to (rows-3): content area
        // - row (rows-2): modeline
        // - row (rows-1): minibuffer
        let content_rows = self.rows.saturating_sub(2) as usize;
        let modeline_row = self.rows.saturating_sub(2);
        let minibuffer_row = self.rows.saturating_sub(1);

        // Collect render data before borrowing text mutably
        let mut content_spans: Vec<(String, GlyphonColor)> = Vec::new();
        let mut primary_cursor_pos: Option<(u16, u16)> = None;
        let mut secondary_cursor_positions: Vec<(u16, u16)> = Vec::new();
        let mut selection_rects: Vec<(u16, u16, u16)> = Vec::new(); // (col, row, width)

        let syntax_theme = SyntaxTheme::modus_operandi();

        for window in self.state.windows.iter() {
            let buffer = match self.state.buffers.get(window.buffer_id) {
                Some(b) => b,
                None => continue,
            };

            use crate::core::rope_ext::RopeExt;

            // Build content spans line by line with syntax highlighting
            for row in 0..content_rows {
                let line_idx = window.scroll_line + row;
                if line_idx < buffer.text.len_lines() {
                    let line = buffer.text.line(line_idx);
                    let line_str: String = line.chars().take(self.cols as usize).collect();
                    let trimmed = line_str.trim_end_matches('\n');
                    let expanded = expand_tabs(trimmed, TAB_WIDTH);

                    let highlights = buffer.highlights_for_line(line_idx);
                    let line_start_byte = buffer.text.line_to_byte(line_idx);

                    if highlights.is_empty() {
                        content_spans.push((expanded, theme.foreground));
                    } else {
                        build_highlighted_spans(
                            trimmed,
                            &highlights,
                            line_start_byte,
                            &syntax_theme,
                            theme.foreground,
                            &mut content_spans,
                        );
                    }
                    content_spans.push(("\n".to_string(), theme.foreground));
                } else {
                    content_spans.push(("~\n".to_string(), GlyphonColor::rgb(128, 128, 128)));
                }
            }

            // Collect all cursor positions and selection regions
            for (i, cursor) in window.cursors.all_cursors().enumerate() {
                let cursor_pos = buffer.text.char_to_position(cursor.position);
                let cursor_line = cursor_pos.line;
                let cursor_char_col = cursor_pos.column;

                // Check if cursor is visible
                if cursor_line >= window.scroll_line
                    && cursor_line < window.scroll_line + content_rows
                {
                    let visual_row = (cursor_line - window.scroll_line) as u16;
                    // Convert char column to visual column (accounting for tabs)
                    let line_text: String = buffer.text.line(cursor_line).chars().collect();
                    let visual_col = char_col_to_visual_col(&line_text, cursor_char_col) as u16;
                    let grid_pos = (visual_col, visual_row);

                    if i == 0 {
                        primary_cursor_pos = Some(grid_pos);
                    } else {
                        secondary_cursor_positions.push(grid_pos);
                    }
                }

                // Collect selection region if mark is active
                if let Some((start, end)) = cursor.region() {
                    let start_pos = buffer.text.char_to_position(start);
                    let end_pos = buffer.text.char_to_position(end);

                    // For each visible line, calculate selection rectangle
                    for line in start_pos.line..=end_pos.line {
                        if line < window.scroll_line || line >= window.scroll_line + content_rows {
                            continue;
                        }

                        let visual_row = (line - window.scroll_line) as u16;
                        let line_text: String = buffer.text.line(line).chars().collect();
                        let line_len = line_text.chars().count().saturating_sub(1); // Exclude newline

                        let sel_start_char_col = if line == start_pos.line {
                            start_pos.column
                        } else {
                            0
                        };

                        let sel_end_char_col = if line == end_pos.line {
                            end_pos.column
                        } else {
                            line_len
                        };

                        if sel_end_char_col > sel_start_char_col {
                            // Convert char columns to visual columns
                            let visual_start =
                                char_col_to_visual_col(&line_text, sel_start_char_col) as u16;
                            let visual_end =
                                char_col_to_visual_col(&line_text, sel_end_char_col) as u16;
                            let width = visual_end.saturating_sub(visual_start);
                            if width > 0 {
                                selection_rects.push((visual_start, visual_row, width));
                            }
                        }
                    }
                }
            }
        }

        // Build modeline text
        let modeline_text = self.build_modeline_text();

        // Build minibuffer text
        let minibuffer_text = if self.state.minibuffer.is_active() {
            self.state.minibuffer.display()
        } else if let Some(ref msg) = self.state.message {
            msg.clone()
        } else if self.state.key_resolver.is_pending() {
            self.state.key_resolver.pending_display().to_string()
        } else {
            String::new()
        };

        // Create selection rectangle bind groups
        let selection_bind_groups: Vec<_> = selection_rects
            .iter()
            .map(|&(col, row, width)| {
                let (x, y) = self.grid_to_pixel(col, row);
                Self::create_rect_bind_group(
                    gpu,
                    RectUniforms {
                        rect: [x, y, width as f32 * self.cell_width, self.cell_height],
                        color: theme.selection,
                        screen_size: [pixel_width, pixel_height],
                        _padding: [0.0, 0.0],
                    },
                )
            })
            .collect();

        // Create primary cursor bind group
        let primary_cursor_bind_group = primary_cursor_pos.map(|(col, row)| {
            let (x, y) = self.grid_to_pixel(col, row);
            Self::create_rect_bind_group(
                gpu,
                RectUniforms {
                    rect: [x, y, self.cell_width, self.cell_height],
                    color: theme.cursor_bg,
                    screen_size: [pixel_width, pixel_height],
                    _padding: [0.0, 0.0],
                },
            )
        });

        // Create secondary cursor bind groups (different color)
        let secondary_cursor_color = [0.5, 0.5, 0.55, 1.0]; // Gray for secondary cursors
        let secondary_cursor_bind_groups: Vec<_> = secondary_cursor_positions
            .iter()
            .map(|&(col, row)| {
                let (x, y) = self.grid_to_pixel(col, row);
                Self::create_rect_bind_group(
                    gpu,
                    RectUniforms {
                        rect: [x, y, self.cell_width, self.cell_height],
                        color: secondary_cursor_color,
                        screen_size: [pixel_width, pixel_height],
                        _padding: [0.0, 0.0],
                    },
                )
            })
            .collect();

        // Create modeline background bind group
        let (modeline_x, modeline_y) = self.grid_to_pixel(0, modeline_row);
        let modeline_bg_bind_group = Self::create_rect_bind_group(
            gpu,
            RectUniforms {
                rect: [modeline_x, modeline_y, pixel_width, self.cell_height],
                color: theme.modeline_bg,
                screen_size: [pixel_width, pixel_height],
                _padding: [0.0, 0.0],
            },
        );

        // Now borrow text mutably for rendering
        let text = match &mut self.text {
            Some(t) => t,
            None => return,
        };

        let gpu = self.gpu.as_ref().unwrap();

        text.viewport.update(
            &gpu.queue,
            Resolution {
                width: gpu_width,
                height: gpu_height,
            },
        );

        // Prepare text buffers - line_height must match cell_height
        let metrics = Metrics::new(FONT_SIZE, CELL_HEIGHT);
        let mut text_buffers: Vec<(GlyphonBuffer, (f32, f32))> = Vec::new();

        // Content buffer at (0, 0) with syntax highlighting
        let mut content_buffer = GlyphonBuffer::new(&mut text.font_system, metrics);
        content_buffer.set_size(
            &mut text.font_system,
            Some(pixel_width),
            Some(content_rows as f32 * self.cell_height),
        );

        let rich_spans: Vec<(&str, Attrs)> = content_spans
            .iter()
            .map(|(text, color)| {
                (
                    text.as_str(),
                    Attrs::new().family(Family::Name(FONT_FAMILY)).color(*color),
                )
            })
            .collect();
        content_buffer.set_rich_text(
            &mut text.font_system,
            rich_spans,
            Attrs::new().family(Family::Name(FONT_FAMILY)),
            Shaping::Advanced,
        );
        text_buffers.push((content_buffer, (0.0, 0.0)));

        // Modeline buffer at modeline_row
        let mut modeline_buffer = GlyphonBuffer::new(&mut text.font_system, metrics);
        modeline_buffer.set_size(
            &mut text.font_system,
            Some(pixel_width),
            Some(self.cell_height),
        );
        modeline_buffer.set_text(
            &mut text.font_system,
            &modeline_text,
            Attrs::new()
                .family(Family::Name(FONT_FAMILY))
                .color(theme.modeline_fg),
            Shaping::Advanced,
        );
        text_buffers.push((modeline_buffer, (0.0, modeline_y)));

        // Minibuffer buffer at minibuffer_row
        let minibuffer_y = minibuffer_row as f32 * self.cell_height;
        let mut minibuffer_buffer = GlyphonBuffer::new(&mut text.font_system, metrics);
        minibuffer_buffer.set_size(
            &mut text.font_system,
            Some(pixel_width),
            Some(self.cell_height),
        );
        minibuffer_buffer.set_text(
            &mut text.font_system,
            &minibuffer_text,
            Attrs::new().family(Family::Name(FONT_FAMILY)),
            Shaping::Advanced,
        );
        text_buffers.push((minibuffer_buffer, (0.0, minibuffer_y)));

        // Prepare text renderer
        text.text_renderer
            .prepare(
                &gpu.device,
                &gpu.queue,
                &mut text.font_system,
                &mut text.atlas,
                &text.viewport,
                text_buffers.iter().map(|(buf, pos)| TextArea {
                    buffer: buf,
                    left: pos.0,
                    top: pos.1,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: 0,
                        top: 0,
                        right: gpu_width as i32,
                        bottom: gpu_height as i32,
                    },
                    default_color: theme.foreground,
                    custom_glyphs: &[],
                }),
                &mut text.swash_cache,
            )
            .unwrap();

        // Render
        let mut encoder = gpu
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
                            r: theme.background[0] as f64,
                            g: theme.background[1] as f64,
                            b: theme.background[2] as f64,
                            a: theme.background[3] as f64,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Draw rectangles first (behind text)
            pass.set_pipeline(&gpu.rect_pipeline);

            // Modeline background
            pass.set_bind_group(0, &modeline_bg_bind_group, &[]);
            pass.draw(0..6, 0..1);

            // Selection regions (behind cursors)
            for bind_group in &selection_bind_groups {
                pass.set_bind_group(0, bind_group, &[]);
                pass.draw(0..6, 0..1);
            }

            // Secondary cursors (gray)
            for bind_group in &secondary_cursor_bind_groups {
                pass.set_bind_group(0, bind_group, &[]);
                pass.draw(0..6, 0..1);
            }

            // Primary cursor (on top)
            if let Some(ref bind_group) = primary_cursor_bind_group {
                pass.set_bind_group(0, bind_group, &[]);
                pass.draw(0..6, 0..1);
            }

            // Draw text on top
            text.text_renderer
                .render(&text.atlas, &text.viewport, &mut pass)
                .unwrap();
        }

        gpu.queue.submit(Some(encoder.finish()));
        frame.present();

        text.atlas.trim();
    }

    fn build_modeline_text(&self) -> String {
        use crate::core::rope_ext::RopeExt;

        let buffer = self.state.current_buffer();
        let window = self.state.current_window();

        let buffer_name = buffer.map(|b| b.name.as_str()).unwrap_or("[No buffer]");
        let modified = buffer
            .map(|b| if b.modified { "**" } else { "--" })
            .unwrap_or("--");
        let readonly = buffer
            .map(|b| if b.read_only { "%%" } else { "--" })
            .unwrap_or("--");

        let mark_indicator = window
            .map(|w| {
                if w.cursors.primary.mark_active {
                    " Mark"
                } else {
                    ""
                }
            })
            .unwrap_or("");

        let (line, col) = match (buffer, window) {
            (Some(b), Some(w)) => {
                let pos = b.text.char_to_position(w.cursors.primary.position);
                (pos.line + 1, pos.column + 1)
            }
            _ => (1, 1),
        };

        let left = format!(
            "-{}:{}- {}{} ",
            modified, readonly, buffer_name, mark_indicator
        );
        let right = format!(" L{}:C{} ", line, col);

        let padding = (self.cols as usize).saturating_sub(left.len() + right.len());
        let dashes = "-".repeat(padding);

        format!("{}{}{}", left, dashes, right)
    }

    fn handle_key(&mut self, event: WinitKeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }

        let key_event = match self.convert_key_event(&event) {
            Some(k) => k,
            None => return,
        };

        self.state.handle_key(key_event);
    }

    fn convert_key_event(&self, event: &WinitKeyEvent) -> Option<KeyEvent> {
        let mut modifiers = Modifiers::empty();

        if self.modifiers.control_key() {
            modifiers |= Modifiers::CTRL;
        }
        // Cmd (Super) maps to Meta for Emacs-style keybindings on macOS
        if self.modifiers.super_key() {
            modifiers |= Modifiers::META;
        }
        // Alt/Option maps to Super (or could be used for other purposes)
        if self.modifiers.alt_key() {
            modifiers |= Modifiers::SUPER;
        }
        if self.modifiers.shift_key() {
            modifiers |= Modifiers::SHIFT;
        }

        use crate::keybinding::key::Key as EnacsKey;

        let key = match &event.logical_key {
            Key::Named(named) => match named {
                NamedKey::Backspace => EnacsKey::Backspace,
                NamedKey::Tab => EnacsKey::Tab,
                NamedKey::Enter => EnacsKey::Enter,
                NamedKey::Escape => EnacsKey::Escape,
                NamedKey::Space => EnacsKey::Char(' '),
                NamedKey::ArrowUp => EnacsKey::Up,
                NamedKey::ArrowDown => EnacsKey::Down,
                NamedKey::ArrowLeft => EnacsKey::Left,
                NamedKey::ArrowRight => EnacsKey::Right,
                NamedKey::Home => EnacsKey::Home,
                NamedKey::End => EnacsKey::End,
                NamedKey::PageUp => EnacsKey::PageUp,
                NamedKey::PageDown => EnacsKey::PageDown,
                NamedKey::Insert => EnacsKey::Insert,
                NamedKey::Delete => EnacsKey::Delete,
                NamedKey::F1 => EnacsKey::F(1),
                NamedKey::F2 => EnacsKey::F(2),
                NamedKey::F3 => EnacsKey::F(3),
                NamedKey::F4 => EnacsKey::F(4),
                NamedKey::F5 => EnacsKey::F(5),
                NamedKey::F6 => EnacsKey::F(6),
                NamedKey::F7 => EnacsKey::F(7),
                NamedKey::F8 => EnacsKey::F(8),
                NamedKey::F9 => EnacsKey::F(9),
                NamedKey::F10 => EnacsKey::F(10),
                NamedKey::F11 => EnacsKey::F(11),
                NamedKey::F12 => EnacsKey::F(12),
                _ => return None,
            },
            Key::Character(c) => {
                let ch = c.chars().next()?;

                // Normalize modifiers to match terminal behavior:
                // 1. For uppercase letters with Ctrl/Meta: convert to lowercase, keep SHIFT
                // 2. For uppercase letters without Ctrl/Meta: strip SHIFT (used to produce uppercase)
                // 3. For shifted symbols (like <, >, !, @, etc.): strip SHIFT
                //    because shift was used to produce the character, not as a modifier

                if ch.is_ascii_uppercase() {
                    if modifiers.contains(Modifiers::CTRL) || modifiers.contains(Modifiers::META) {
                        // Uppercase letter with Ctrl/Meta -> lowercase + SHIFT
                        modifiers |= Modifiers::SHIFT;
                        EnacsKey::Char(ch.to_ascii_lowercase())
                    } else {
                        // Regular uppercase letter -> strip SHIFT (it produced the uppercase)
                        modifiers.remove(Modifiers::SHIFT);
                        EnacsKey::Char(ch)
                    }
                } else if is_shifted_symbol(ch) {
                    // Shifted symbol -> strip SHIFT (it was used to produce the char)
                    modifiers.remove(Modifiers::SHIFT);
                    EnacsKey::Char(ch)
                } else {
                    EnacsKey::Char(ch)
                }
            }
            _ => return None,
        };

        Some(KeyEvent { key, modifiers })
    }
}

impl ApplicationHandler for GuiApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = WindowAttributes::default()
            .with_title("Enacs")
            .with_inner_size(LogicalSize::new(1000, 700));

        let window = Arc::new(event_loop.create_window(attrs).unwrap());

        self.init_gpu(window.clone());

        let size = window.inner_size();
        self.cols = (size.width as f32 / self.cell_width) as u16;
        self.rows = (size.height as f32 / self.cell_height) as u16;
        self.state
            .set_dimensions(self.cols, self.rows.saturating_sub(2));

        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                self.resize(size);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_key(event);
                if self.state.should_quit {
                    event_loop.exit();
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }
            WindowEvent::Focused(focused) => {
                if focused {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
