/// 頂点データの構造を定義する。
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![
            0 => Float32x3, // position:   location(0)
            1 => Float32x2  // tex_coords: location(1)
        ];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}

/// 四角形の頂点データを生成する関数
pub fn create_quad() -> (Vec<Vertex>, Vec<u32>) {
    let vertices = vec![
        Vertex {
            position: [-1.0, 1.0, 0.0],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [-1.0, -1.0, 0.0],
            tex_coords: [0.0, 1.0],
        },
        Vertex {
            position: [1.0, -1.0, 0.0],
            tex_coords: [1.0, 1.0],
        },
        Vertex {
            position: [1.0, 1.0, 0.0],
            tex_coords: [1.0, 0.0],
        },
    ];

    let indices = vec![0, 1, 2, 0, 2, 3];

    (vertices, indices)
}
