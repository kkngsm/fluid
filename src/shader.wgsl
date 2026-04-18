
// グループ1のバインド番号0で解像度情報を受け取る（将来用）。
// var<uniform> は「全ての並列処理で共通（読み取り専用）のデータ」という意味。
@group(1) @binding(0)
var<uniform> resolution: vec2<f32>;

/// 頂点バッファから入力されるデータの構造。
/// location(N) は、Rust側の VertexBufferLayout と対応している必要がある。
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

/// 頂点シェーダーから出力され、フラグメントシェーダーへと渡されるデータの構造。
struct VertexOutput {
    /// builtin(position) は GPU が座標計算に使用する特別な予約語（クリップ座標系）。
    @builtin(position) clip_position: vec4<f32>,
    /// 次のステージ（フラグメントシェーダー）で受け取るためのカスタム属性。
    @location(0) color: vec3<f32>,
};

/// 頂点シェーダー (@vertex)
/// 各頂点ごとに実行され、頂点の座標を決定する。
@vertex
fn vs_main(vin: VertexInput) -> VertexOutput {
    var vout: VertexOutput;
    // Rust側から送られた色をそのままフラグメントシェーダーへ渡す。
    vout.color = vin.color;
    // 3次元座標(vec3)を、クリップ空間(vec4)に変換。
    // w成分を1.0にすることで、同次座標系として位置を表現する。
    vout.clip_position = vec4<f32>(vin.position, 1.0);
    return vout;
}

/// フラグメントシェーダー (@fragment)
/// 画面上の各ピクセル（厳密にはフラグメント）ごとに実行され、最終的な色を決定する。
@fragment
fn fs_main(fin: VertexOutput) -> @location(0) vec4<f32> {
    // 頂点シェーダーで出力された色は、ピクセル間で自動的に補完（線形補完）される。
    // アルファ成分を 1.0 (不透明) として最終的な色を出力。
    return vec4<f32>(fin.color, 1.0);
}