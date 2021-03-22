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

layout(push_constant) uniform Push {
    uint push;
};

void main() {
    uint gid = gl_GlobalInvocationID.x;
    if (gid >= inp.length()) return;
    outp[gid] = inp[gid] * inp[gid] * push;
}
"#;

fn main() -> Result<()> {
    let mut engine = Engine::new(true)?;
    let shader = engine.glsl(SHADER_SRC)?;
    const INVOKE_X: u32 = 50;
    let mut data: Vec<u32> = (0..).take((LOCAL_SIZE_X * INVOKE_X) as _).collect();

    let input = engine.buffer::<u32>(data.len())?;
    let output = engine.buffer::<u32>(data.len())?;
    engine.write::<u32>(input, &data)?;
    engine.run(shader, input, output, INVOKE_X, 1, 1, &5u32.to_le_bytes())?;
    engine.read::<u32>(output, &mut data)?;

    dbg!(data);

    Ok(())
}
