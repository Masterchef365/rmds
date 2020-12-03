# Run My Damn Shader
Really simple tool to play with compute shaders in basic scenarios

## Goals:
1. Ease of use
    * Should be frictionless to implement an idea in a shader and integrate it with code elsewhere
    * Don't silo the user into one usecase, but also don't be so general as to make the abstraction pointless
2. Safety
    * Throw clear error messages when ex. data input mismatches buffer size
2. Speed
    * We should be able to use this to see "what's easily attainable" in terms of compute times.

## Non-goals: 
* These may conflict with the ideas lol
* Fitting every use-case (I.E. you wouldn't want to rebuild TensorSludge on top of this)
* Squeezing every ounce of performance out.

## Ideas:
* Name the buffers! This is useful for error messages
* Buffer slicing options for `run()` and others, so that you can create a massive buffer and update slices into it.
* Separate in/out buffers? Some tasks benefit from that. 
    * If needed, the present case case can be kludged by using half the buffer for read and half for write, but since we don't have a sliced read/write transfers will be even less efficient.
* It would be nice to have chaining commands (Buffer barriers instead of `vkQueueWaitIdle()`)
* Explicit staging buffers, since having read/write remap every time is just bad
    * Maybe only keep one read/write transfer buffer around, but its size increases as needed. 
    * Or maybe just don't bother using different sized transfer buffers to cover the whole thing and just stream data through a fixed sized transfer buffer
* Use [PushDescriptors](https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VK_KHR_push_descriptor.html)?
* Compat with klystron for realtime 2D/3D visualization. 
    * Switch Klystron over to `vk_core`
    * Need some mechanism for obtaining the buffer once we share an instance. Warning: Stalling the _only_ queue is a bad idea, esp. for VR... Should use barriers
* Auto-update - Wait for file updates and re-runs shader... Honestly that should 
