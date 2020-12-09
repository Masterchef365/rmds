use anyhow::Result;
use rmds::Engine;

const LOCAL_SIZE_X: u32 = 16;
const SHADER_SRC: &str = r#"
#version 450
layout (local_size_x = 16) in;

layout(binding = 0) readonly buffer InputData {
    uint inp[];
};

layout(binding = 1) writeonly buffer OutputData {
    uint outp[];
};

void main() {
    uint gid = gl_GlobalInvocationID.x;
    if (gid >= inp.length()) return;
    outp[gid] = inp[gid] * inp[gid];
}
"#;

fn main() -> Result<()> {
    let mut engine = Engine::new(true)?;
    let shader = engine.glsl(SHADER_SRC)?;
    const INVOKE_X: u32 = 50;
    let mut data: Vec<u32> = (0..).take((LOCAL_SIZE_X * INVOKE_X) as _).collect();
    let data_mut: &mut [u8] = bytemuck::cast_slice_mut(&mut data);

    let input = engine.buffer(data_mut.len() as _)?;
    let output = engine.buffer(data_mut.len() as _)?;
    engine.write(input, data_mut)?;
    engine.run(shader, input, output, INVOKE_X, 1, 1)?;
    engine.read(output, data_mut)?;

    dbg!(data);

    Ok(())
}
