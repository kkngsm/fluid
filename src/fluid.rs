pub struct Fluid {
    size: usize,
    dt: f32,
    diff: f32,
    visc: f32,

    pub density: Vec<f32>,
    pub velocity_x: Vec<f32>,
    pub velocity_y: Vec<f32>,

    density0: Vec<f32>,
    velocity_x0: Vec<f32>,
    velocity_y0: Vec<f32>,
}

impl Fluid {
    pub fn new(size: usize, dt: f32, diffusion: f32, viscosity: f32) -> Self {
        let num_cells = (size + 2) * (size + 2);
        Self {
            size,
            dt,
            diff: diffusion,
            visc: viscosity,
            density: vec![0.0; num_cells],
            velocity_x: vec![0.0; num_cells],
            velocity_y: vec![0.0; num_cells],
            density0: vec![0.0; num_cells],
            velocity_x0: vec![0.0; num_cells],
            velocity_y0: vec![0.0; num_cells],
        }
    }

    pub fn step(&mut self) {
        // ここに Stable Fluid のソルバー（拡散、移流、射影）を実装していきます
        // 今回はまずデータの受け皿として機能させます
        
        // 簡易的な減衰処理（テスト用）
        for d in self.density.iter_mut() {
            *d *= 0.99;
        }
    }

    pub fn add_density(&mut self, x: usize, y: usize, amount: f32) {
        let size = self.size;
        if x > 0 && x <= size && y > 0 && y <= size {
            let index = (x) + (y) * (size + 2);
            self.density[index] += amount;
        }
    }

    pub fn add_velocity(&mut self, x: usize, y: usize, px: f32, py: f32) {
        let size = self.size;
        if x > 0 && x <= size && y > 0 && y <= size {
            let index = (x) + (y) * (size + 2);
            self.velocity_x[index] += px;
            self.velocity_y[index] += py;
        }
    }

    pub fn get_density(&self, x: usize, y: usize) -> f32 {
        let index = (x + 1) + (y + 1) * (self.size + 2);
        self.density[index]
    }
}
