use cuda_builder::*;

fn main() {
    CudaBuilder::new("../gpu")
        .copy_to("gpu.ptx")
        .release(true)
        .build()
        .unwrap();
}
