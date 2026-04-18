/// 頂点データの構造を定義する。
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    const ATTRS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![
            0 => Float32x3, // position: location(0)
            1 => Float32x3  // color:    location(1)
        ];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}

/// 格子状の頂点データを生成する関数
pub fn create_grid(rows: u32, cols: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for y in 0..=rows {
        for x in 0..=cols {
            // -1.0 から 1.0 の範囲に正規化
            let px = (x as f32 / cols as f32) * 2.0 - 1.0;
            let py = (y as f32 / rows as f32) * 2.0 - 1.0;
            
            vertices.push(Vertex {
                position: [px, py, 0.0],
                // グラデーション: x方向を赤、y方向を緑に割り当てる
                color: [x as f32 / cols as f32, y as f32 / rows as f32, 0.5],
            });
        }
    }

    for y in 0..rows {
        for x in 0..cols {
            let root = y * (cols + 1) + x;
            // 2つの三角形で1つのセル（四角形）を形成
            indices.push(root);
            indices.push(root + 1);
            indices.push(root + cols + 1);

            indices.push(root + 1);
            indices.push(root + cols + 2);
            indices.push(root + cols + 1);
        }
    }

    (vertices, indices)
}
