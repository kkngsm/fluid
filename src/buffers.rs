use wgpu::{util::DeviceExt, Buffer, Device, IndexFormat, ShaderStages};

use crate::vertex::{INDICES, VERTICES};

/// GPU上で動作するために必要なすべてのメモリバッファとデータ転送設定を管理する構造体。
pub struct Buffers {
    /// 頂点データ（位置、色など）を格納するバッファ
    pub vertex: VertexBuffer,
    /// 頂点を描画する順番（インデックス）を格納するバッファ。メモリ節約と高速化に寄与。
    pub index: IndexBuffer,
    /// シェーダーに渡す追加データ（定数、行列、テクスチャなど）のリスト。
    pub bind_groups: Vec<BindGroup>,
}

impl Buffers {
    /// デバイスを使用して初期バッファを作成する。
    pub fn new(device: &Device) -> Self {
        // 頂点とインデックスをGPUメモリに転送
        let vertex = VertexBuffer::new(device, VERTICES);
        let index = IndexBuffer::new(device, INDICES);
        let bind_groups = vec![];
        Self {
            vertex,
            index,
            bind_groups,
        }
    }

    /// 新しいバインドグループを追加するためのヘルパーメソッド。
    pub fn bind_group<T: bytemuck::Pod>(mut self, bind_group: BindGroup) -> Self {
        self.bind_groups.push(bind_group);
        self
    }
}

/// 頂点データを保持する専用のラッパー。
pub struct VertexBuffer {
    pub buffer: Buffer,
}

impl VertexBuffer {
    pub fn new<T: bytemuck::Pod>(device: &Device, vertices: &[T]) -> Self {
        // wgpu::util::DeviceExt を使用して、作成と同時にデータを書き込む。
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            // このバッファが頂点データとして使われることをGPUに伝える。
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self { buffer }
    }
}

/// インデックスのデータ型（u16 または u32）から wgpu の IndexFormat を取得するためのトレイト。
trait FormatDetect {
    fn to_index_format() -> IndexFormat;
}
impl FormatDetect for u32 {
    fn to_index_format() -> IndexFormat {
        IndexFormat::Uint32
    }
}
impl FormatDetect for u16 {
    fn to_index_format() -> IndexFormat {
        IndexFormat::Uint16
    }
}

/// インデックス（描画順序）データを保持する専用のラッパー。
pub struct IndexBuffer {
    pub buffer: Buffer,
    pub format: IndexFormat,
}
impl IndexBuffer {
    #[allow(private_bounds)]
    pub fn new<T: bytemuck::Pod + FormatDetect>(device: &Device, indices: &[T]) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            // インデックスデータとして使用されることを指定。
            usage: wgpu::BufferUsages::INDEX,
        });
        Self {
            buffer,
            format: T::to_index_format(),
        }
    }
}

/// バインドグループ内の各リソース（設計図側）の性質を定義する。
struct BindGroupLayoutEntry {
    /// どのシェーダー（頂点、フラグメント、計算）からこのデータにアクセスできるか。
    pub visibility: ShaderStages,
    /// データの種類（Uniformバッファ、ストレージバッファ、サンプラー、テクスチャ等）。
    pub ty: wgpu::BindingType,
    /// 配列の場合の要素数。
    pub count: Option<std::num::NonZeroU32>,
}
impl BindGroupLayoutEntry {
    fn to_wgpu(&self, binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding, // シェーダー内の @binding(N) と対応
            visibility: self.visibility,
            ty: self.ty,
            count: self.count,
        }
    }
}

/// バッファ実体と、そのバッファがどういう性質を持つかの定義をペアにする。
pub struct BindGroupEntry {
    buffer: wgpu::Buffer,
    layout: BindGroupLayoutEntry,
}

/// 「バインドグループ」はシェーダーに渡すデータのセット。
/// 頂点データ以外のあらゆる情報（変換行列、光の情報、テクスチャなど）をまとめる。
pub struct BindGroup {
    label: String,

    entries: Vec<BindGroupEntry>,
    entry_labels: Vec<String>,
    entry_layout: Vec<wgpu::BindGroupLayoutEntry>,
}

impl BindGroup {
    pub fn new(label: impl ToString) -> Self {
        Self {
            label: label.to_string(),

            entries: vec![],
            entry_labels: vec![],
            entry_layout: vec![],
        }
    }

    /// バインドグループに要素を追加する。
    pub fn insert(mut self, label: String, entry: BindGroupEntry) -> Self {
        // レイアウト情報を構築。
        self.entry_layout
            .push(entry.layout.to_wgpu(self.entries.len() as u32));
        self.entries.push(entry);
        self.entry_labels.push(label);
        self
    }

    /// 「バインドグループ・レイアウト」を作成。これはデータの設計図であり、
    /// パイプライン作成時に「どういうデータが渡される予定か」をGPUに教えるために使う。
    pub fn bind_group_layout(&self, device: &Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &self.entry_layout,
            label: Some(&format!("{}_layout", self.label)),
        })
    }

    /// 「バインドグループ」本体を作成。実際のGPU上のバッファを設計図に当てはめる。
    pub fn bind_group(&self, device: &Device, layout: &wgpu::BindGroupLayout) -> wgpu::BindGroup {
        let entries = self
            .entries
            .iter()
            .enumerate()
            .map(|(binding, entry)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: entry.buffer.as_entire_binding(),
            })
            .collect::<Vec<_>>();
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &entries,
            label: Some(&self.label),
        })
    }

    /// レイアウト（設計図）とグループ（実体）を同時に作成して返す。
    pub fn group_and_layout(&self, device: &Device) -> (wgpu::BindGroup, wgpu::BindGroupLayout) {
        let layout = self.bind_group_layout(device);
        let group = self.bind_group(device, &layout);
        (group, layout)
    }
}
