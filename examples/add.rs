use anyhow::Result;
use rmds::Engine;

const LOCAL_SIZE_X: u32 = 16;
const SHADER_SRC: &str = r#"
#version 450
layout (local_size_x = 16) in;

layout(binding = 0) buffer Data {
    uint data[];
};

void main() {
    uint gid = gl_GlobalInvocationID.x;
    if (gid >= data.length()) return;
    data[gid] += 5;
}
"#;

fn main() -> Result<()> {
    let mut engine = Engine::new(true)?;
    let shader = engine.glsl(SHADER_SRC)?;
    const INVOKE_X: u32 = 50;
    let mut data: Vec<u32> = (0..).take((LOCAL_SIZE_X * INVOKE_X) as _).collect();
    let data_mut: &mut [u8] = bytemuck::cast_slice_mut(&mut data);

    let buffer = engine.buffer(data_mut.len())?;
    engine.write(buffer, data_mut)?;
    engine.run(shader, buffer, INVOKE_X, 1, 1)?;
    engine.read(buffer, data_mut)?;

    dbg!(data);

    Ok(())
}
