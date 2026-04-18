/// 頂点データの構造を定義する。
/// `repr(C)` は、Rustのコンパイラが勝手に構造体のメモリレイアウトを変更しないように（C言語と同じ並びに）するために必須。
#[repr(C)]
/// `bytemuck::Pod` と `Zeroable` は、この構造体を安全にバイト列に変換（キャスト）してGPUに送れるようにするために必要。
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    /// 位置情報 (x, y, z)
    position: [f32; 3],
    /// 色情報 (r, g, b)
    color: [f32; 3],
}

impl Vertex {
    /// 頂点の各要素（属性）がシェーダー内のどの場所（location）に対応するかを定義する。
    const ATTRS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![
            0 => Float32x3, // position: location(0)
            1 => Float32x3  // color:    location(1)
        ];

    /// wgpuに対して、頂点データのメモリ上の並び（レイアウト）を伝えるための設定。
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            // 頂点1つ分のサイズ
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            // 各頂点ごとにデータを進める（インスタンス描画の場合はここを変える）
            step_mode: wgpu::VertexStepMode::Vertex,
            // 属性の定義
            attributes: &Self::ATTRS,
        }
    }
}

/// 描画に使用する具体的な頂点データ。
/// ここでは画面いっぱいに広がる四角形（2つの三角形）を定義している。
pub const VERTICES: &[Vertex] = &[
    // 左上
    Vertex {
        position: [-1.0, 1.0, 0.0],
        color: [0.0, 1.0, 0.0],
    },
    // 左下
    Vertex {
        position: [-1.0, -1.0, 0.0],
        color: [0.0, 0.0, 0.0],
    },
    // 右下
    Vertex {
        position: [1.0, -1.0, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    // 右上
    Vertex {
        position: [1.0, 1.0, 0.0],
        color: [1.0, 1.0, 0.0],
    },
];

/// インデックスデータ（頂点をどの順番で結ぶか）。
/// 0-1-2 で一つの三角形、0-2-3 でもう一つの三角形を形成し、合わせて四角形になる。
#[rustfmt::skip]
pub const INDICES: &[u16] = &[
    0, 1, 2,
    0, 2, 3,
];
