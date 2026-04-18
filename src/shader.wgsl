
struct AspectUniform {
    ratio: f32,
};

@group(0) @binding(0)
var<uniform> aspect: AspectUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(vin: VertexInput) -> VertexOutput {
    var vout: VertexOutput;
    vout.color = vin.color;
    
    var pos = vin.position;
    // アスペクト比を考慮して座標を調整（正方形を維持）
    if (aspect.ratio > 1.0) {
        // 横長の場合、x座標を縮小
        pos.x = pos.x / aspect.ratio;
    } else {
        // 縦長の場合、y座標にアスペクト比を掛ける（yは縮小される）
        pos.y = pos.y * aspect.ratio;
    }
    
    vout.clip_position = vec4<f32>(pos, 1.0);
    return vout;
}

@fragment
fn fs_main(fin: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(fin.color, 1.0);
}
