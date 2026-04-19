use wgpu::{util::DeviceExt, Buffer, Device, IndexFormat, ShaderStages, Queue};

/// GPU上で動作するために必要なすべてのメモリバッファとデータ転送設定を管理する構造体。
pub struct Buffers {
    /// 頂点データ（位置、色など）を格納するバッファ
    pub vertex: VertexBuffer,
    /// 頂点を描画する順番（インデックス）を格納するバッファ。
    pub index: IndexBuffer,
    /// シェーダーに渡す追加データ（定数、行列、テクスチャなど）のリスト。
    pub bind_groups: Vec<BindGroup>,
}

impl Buffers {
    pub fn new(device: &Device, vertices: &[crate::vertex::Vertex], indices: &[u32]) -> Self {
        let vertex = VertexBuffer::new(device, vertices);
        let index = IndexBuffer::new(device, indices);
        let bind_groups = vec![];
        Self {
            vertex,
            index,
            bind_groups,
        }
    }

    pub fn add_bind_group(mut self, bind_group: BindGroup) -> Self {
        self.bind_groups.push(bind_group);
        self
    }
}

pub struct VertexBuffer {
    pub buffer: Buffer,
}

impl VertexBuffer {
    pub fn new<T: bytemuck::Pod>(device: &Device, vertices: &[T]) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        Self { buffer }
    }

    pub fn update<T: bytemuck::Pod>(&self, queue: &Queue, vertices: &[T]) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(vertices));
    }
}

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

pub struct IndexBuffer {
    pub buffer: Buffer,
    pub format: IndexFormat,
    pub num_indices: u32,
}
impl IndexBuffer {
    #[allow(private_bounds)]
    pub fn new<T: bytemuck::Pod + FormatDetect>(device: &Device, indices: &[T]) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        Self {
            buffer,
            format: T::to_index_format(),
            num_indices: indices.len() as u32,
        }
    }
}

pub struct BindGroupLayoutEntry {
    pub visibility: ShaderStages,
    pub ty: wgpu::BindingType,
    pub count: Option<std::num::NonZeroU32>,
}
impl BindGroupLayoutEntry {
    fn to_wgpu(&self, binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: self.visibility,
            ty: self.ty,
            count: self.count,
        }
    }

    pub fn uniform() -> Self {
        Self {
            visibility: ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

    pub fn texture() -> Self {
        Self {
            visibility: ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }
    }

    pub fn sampler() -> Self {
        Self {
            visibility: ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        }
    }
}

pub enum BindGroupResource {
    Buffer(wgpu::Buffer),
    Texture(wgpu::TextureView),
    Sampler(wgpu::Sampler),
}

pub struct BindGroupEntry {
    pub resource: BindGroupResource,
    pub layout: BindGroupLayoutEntry,
}

impl BindGroupEntry {
    pub fn uniform<T: bytemuck::Pod>(device: &Device, data: T) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::bytes_of(&data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            resource: BindGroupResource::Buffer(buffer),
            layout: BindGroupLayoutEntry::uniform(),
        }
    }

    pub fn texture(view: wgpu::TextureView) -> Self {
        Self {
            resource: BindGroupResource::Texture(view),
            layout: BindGroupLayoutEntry::texture(),
        }
    }

    pub fn sampler(sampler: wgpu::Sampler) -> Self {
        Self {
            resource: BindGroupResource::Sampler(sampler),
            layout: BindGroupLayoutEntry::sampler(),
        }
    }

    pub fn update_buffer<T: bytemuck::Pod>(&self, queue: &Queue, data: T) {
        if let BindGroupResource::Buffer(buffer) = &self.resource {
            queue.write_buffer(buffer, 0, bytemuck::bytes_of(&data));
        }
    }
}

pub struct BindGroup {
    pub label: String,
    pub entries: Vec<BindGroupEntry>,
    pub entry_layout: Vec<wgpu::BindGroupLayoutEntry>,
}

impl BindGroup {
    pub fn new(label: impl ToString) -> Self {
        Self {
            label: label.to_string(),
            entries: vec![],
            entry_layout: vec![],
        }
    }

    pub fn insert(mut self, entry: BindGroupEntry) -> Self {
        self.entry_layout
            .push(entry.layout.to_wgpu(self.entries.len() as u32));
        self.entries.push(entry);
        self
    }

    pub fn bind_group_layout(&self, device: &Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &self.entry_layout,
            label: Some(&format!("{}_layout", self.label)),
        })
    }

    pub fn bind_group(&self, device: &Device, layout: &wgpu::BindGroupLayout) -> wgpu::BindGroup {
        let entries = self
            .entries
            .iter()
            .enumerate()
            .map(|(binding, entry)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: match &entry.resource {
                    BindGroupResource::Buffer(buffer) => buffer.as_entire_binding(),
                    BindGroupResource::Texture(view) => wgpu::BindingResource::TextureView(view),
                    BindGroupResource::Sampler(sampler) => wgpu::BindingResource::Sampler(sampler),
                },
            })
            .collect::<Vec<_>>();
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &entries,
            label: Some(&self.label),
        })
    }
}
