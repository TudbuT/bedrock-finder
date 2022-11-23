use cuda_builder::CudaBuilder;

fn main() {
    CudaBuilder::new("../gpu")
        .copy_to("gpu.ptx")
        .release(true)
        .fma_contraction(true)
        .build()
        .unwrap();
}
