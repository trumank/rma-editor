//! Debug line geometry for rendering fixed-width 3D lines
//!
//! Lines maintain constant screen-space thickness regardless of distance from camera.

use three_d::{
    vec2, vec3, AxisAlignedBoundingBox, ColorTexture, Context, DepthTest, DepthTexture, Effect,
    EffectMaterialId, ElementBuffer, Geometry, GeometryId, Light, Material, MaterialType, Program,
    RenderStates, Srgba, Vec2, Vec3, Vec4, VertexBuffer, Viewer,
};

/// A single debug line with start point, end point, and color.
#[derive(Clone, Copy)]
pub struct DebugLine {
    pub start: Vec3,
    pub end: Vec3,
    pub color: Srgba,
}

/// Geometry for rendering 3D lines with fixed pixel width.
/// Lines maintain constant screen-space thickness regardless of distance from camera.
pub struct DebugLines {
    context: Context,
    /// Line thickness in pixels
    thickness: f32,
    // GPU buffers
    positions: VertexBuffer<Vec3>,
    line_endpoints: VertexBuffer<Vec3>, // The "other" endpoint for each vertex
    vertex_offsets: VertexBuffer<Vec2>, // Which corner of the quad (-1/-1, 1/-1, etc)
    colors: VertexBuffer<Vec4>,
    indices: ElementBuffer<u32>,
    // CPU data for updates
    lines: Vec<DebugLine>,
    aabb: AxisAlignedBoundingBox,
}

impl DebugLines {
    /// Create a new DebugLines geometry with the given pixel thickness.
    pub fn new(context: &Context, thickness: f32) -> Self {
        Self {
            context: context.clone(),
            thickness,
            positions: VertexBuffer::new(context),
            line_endpoints: VertexBuffer::new(context),
            vertex_offsets: VertexBuffer::new(context),
            colors: VertexBuffer::new(context),
            indices: ElementBuffer::new(context),
            lines: Vec::new(),
            aabb: AxisAlignedBoundingBox::EMPTY,
        }
    }

    /// Set the lines to render.
    pub fn set_lines(&mut self, lines: Vec<DebugLine>) {
        self.lines = lines;
        self.update_buffers();
    }

    /// Rebuild GPU buffers and recompute AABB. Call after modifying lines.
    pub fn update_buffers(&mut self) {
        if self.lines.is_empty() {
            self.aabb = AxisAlignedBoundingBox::EMPTY;
            return;
        }

        let num_lines = self.lines.len();
        let num_vertices = num_lines * 4; // 4 vertices per line (quad)
        let num_indices = num_lines * 6; // 6 indices per line (2 triangles)

        let mut positions = Vec::with_capacity(num_vertices);
        let mut line_endpoints = Vec::with_capacity(num_vertices);
        let mut vertex_offsets = Vec::with_capacity(num_vertices);
        let mut colors = Vec::with_capacity(num_vertices);
        let mut indices = Vec::with_capacity(num_indices);

        let mut min = vec3(f32::MAX, f32::MAX, f32::MAX);
        let mut max = vec3(f32::MIN, f32::MIN, f32::MIN);

        for (i, line) in self.lines.iter().enumerate() {
            let color = line.color.to_linear_srgb();
            let base_idx = (i * 4) as u32;

            // Update AABB
            min = vec3(
                min.x.min(line.start.x).min(line.end.x),
                min.y.min(line.start.y).min(line.end.y),
                min.z.min(line.start.z).min(line.end.z),
            );
            max = vec3(
                max.x.max(line.start.x).max(line.end.x),
                max.y.max(line.start.y).max(line.end.y),
                max.z.max(line.start.z).max(line.end.z),
            );

            // Each line becomes a quad with 4 vertices
            // Vertex 0: at start, offset -1 (left side)
            // Vertex 1: at start, offset +1 (right side)
            // Vertex 2: at end, offset +1 (right side)
            // Vertex 3: at end, offset -1 (left side)

            // Vertices at start point
            positions.push(line.start);
            line_endpoints.push(line.end);
            vertex_offsets.push(vec2(0.0, -1.0)); // start, left
            colors.push(color);

            positions.push(line.start);
            line_endpoints.push(line.end);
            vertex_offsets.push(vec2(0.0, 1.0)); // start, right
            colors.push(color);

            // Vertices at end point
            positions.push(line.end);
            line_endpoints.push(line.start);
            vertex_offsets.push(vec2(1.0, 1.0)); // end, right
            colors.push(color);

            positions.push(line.end);
            line_endpoints.push(line.start);
            vertex_offsets.push(vec2(1.0, -1.0)); // end, left
            colors.push(color);

            // Two triangles: 0-1-2 and 0-2-3
            indices.push(base_idx);
            indices.push(base_idx + 1);
            indices.push(base_idx + 2);
            indices.push(base_idx);
            indices.push(base_idx + 2);
            indices.push(base_idx + 3);
        }

        self.positions.fill(&positions);
        self.line_endpoints.fill(&line_endpoints);
        self.vertex_offsets.fill(&vertex_offsets);
        self.colors.fill(&colors);
        self.indices.fill(&indices);

        self.aabb = AxisAlignedBoundingBox::new_with_positions(&[min, max]);
    }

    fn vertex_shader_source() -> String {
        r#"
uniform mat4 viewProjection;
uniform vec2 viewportSize;
uniform float lineThickness;

in vec3 position;      // This vertex's position (start or end of line)
in vec3 lineEndpoint;  // The other endpoint of the line
in vec2 vertexOffset;  // x: 0=start, 1=end; y: -1=left, +1=right
in vec4 color;

out vec4 col;

void main() {
    // Project both endpoints to clip space
    vec4 clipPos = viewProjection * vec4(position, 1.0);
    vec4 clipEnd = viewProjection * vec4(lineEndpoint, 1.0);

    // Convert to NDC
    vec2 ndcPos = clipPos.xy / clipPos.w;
    vec2 ndcEnd = clipEnd.xy / clipEnd.w;

    // Compute line direction in screen space
    vec2 lineDir = ndcEnd - ndcPos;

    // Handle degenerate lines (zero length)
    float len = length(lineDir);
    if (len < 0.0001) {
        lineDir = vec2(1.0, 0.0);
    } else {
        lineDir = lineDir / len;
    }

    // Perpendicular direction
    vec2 perpDir = vec2(-lineDir.y, lineDir.x);

    // Offset in NDC space: thickness in pixels -> NDC
    // NDC range is -1 to 1, so 2 units = viewportSize pixels
    vec2 offset = perpDir * vertexOffset.y * lineThickness / viewportSize;

    // Apply offset to the clip-space position
    vec4 finalClipPos = clipPos;
    finalClipPos.xy += offset * clipPos.w;

    gl_Position = finalClipPos;
    col = color;
}
"#
        .to_string()
    }
}

impl Geometry for DebugLines {
    fn aabb(&self) -> AxisAlignedBoundingBox {
        self.aabb
    }

    fn draw(&self, viewer: &dyn Viewer, program: &Program, render_states: RenderStates) {
        if self.lines.is_empty() {
            return;
        }

        // Set uniforms
        program.use_uniform("viewProjection", viewer.projection() * viewer.view());
        let vp = viewer.viewport();
        program.use_uniform("viewportSize", vec2(vp.width as f32, vp.height as f32));
        program.use_uniform("lineThickness", self.thickness);

        // Bind vertex attributes
        program.use_vertex_attribute("position", &self.positions);
        program.use_vertex_attribute("lineEndpoint", &self.line_endpoints);
        program.use_vertex_attribute("vertexOffset", &self.vertex_offsets);
        program.use_vertex_attribute("color", &self.colors);

        // Draw
        program.draw_elements(render_states, vp, &self.indices);
    }

    fn vertex_shader_source(&self) -> String {
        Self::vertex_shader_source()
    }

    fn id(&self) -> GeometryId {
        // Use an ID in the public range (0x0000 - 0x7FFF)
        GeometryId(0x0100)
    }

    fn render_with_material(
        &self,
        material: &dyn Material,
        viewer: &dyn Viewer,
        lights: &[&dyn Light],
    ) {
        three_d::renderer::render_with_material(&self.context, viewer, self, material, lights)
            .unwrap();
    }

    fn render_with_effect(
        &self,
        material: &dyn Effect,
        viewer: &dyn Viewer,
        lights: &[&dyn Light],
        color_texture: Option<ColorTexture>,
        depth_texture: Option<DepthTexture>,
    ) {
        three_d::renderer::render_with_effect(
            &self.context,
            viewer,
            self,
            material,
            lights,
            color_texture,
            depth_texture,
        )
        .unwrap();
    }
}

impl<'a> IntoIterator for &'a DebugLines {
    type Item = &'a dyn Geometry;
    type IntoIter = std::iter::Once<&'a dyn Geometry>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}

/// Simple material for debug lines (unlit, just vertex colors)
#[derive(Clone, Default)]
pub struct DebugLineMaterial {
    pub render_states: RenderStates,
}

impl DebugLineMaterial {
    pub fn new() -> Self {
        Self {
            render_states: RenderStates {
                depth_test: DepthTest::LessOrEqual,
                ..Default::default()
            },
        }
    }
}

impl Material for DebugLineMaterial {
    fn id(&self) -> EffectMaterialId {
        // Use an ID in the public range (0x0000 - 0x4FFF)
        EffectMaterialId(0x0100)
    }

    fn fragment_shader_source(&self, _lights: &[&dyn Light]) -> String {
        r#"
in vec4 col;
layout (location = 0) out vec4 outColor;

void main() {
    outColor = col;
}
"#
        .to_string()
    }

    fn use_uniforms(&self, _program: &Program, _viewer: &dyn Viewer, _lights: &[&dyn Light]) {
        // No additional uniforms needed
    }

    fn render_states(&self) -> RenderStates {
        self.render_states
    }

    fn material_type(&self) -> MaterialType {
        MaterialType::Opaque
    }
}
