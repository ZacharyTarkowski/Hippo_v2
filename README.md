Rewrite of my previous tft "animation" project from C to Rust. Generally for learning, as I have already given a v1 version of the product to my mother. Though I do plan to do a PCB spin with a 3D printed case, with this code as the backbone.

Uses an STM32F401RCT6 connected to an ILI9341 driven TFT screen with a PIR sensor.

Plays an idle "animation" when the PIR sensor has not triggered (two images shifting back and forth). When the PIR sensor trips, it changes to displaying the active "animation" in the same fashion.

Basic arch:

Uses the RTIC framework for interrupt driven scheduling. 
Main task playing the animation runs off of the ARM system timer monotonic, while the PIR sensor is hooked to an interrupt task to change the state from idle to active or vice versa based on the edge of the PIR output line.
Uses the stm32f4 crate to handle hardware setup and the ILI9341 crate to drive the TFT. Both conform to the Rust embedded HAL which is nice.

Images are stored on the on-chip flash. The build script takes paths to the images from rle_config.toml and converts them to 16 bit 6-5-6 RGB and then run-length encodes them. The actual stm32 code grabs the encoded binaries and includes them in the target binary as u16 arrays, which are then read out as part of the ILI9341 draw.

The most interesting piece of the project is probably the custom iterator for the run-length encoded images, which I had to create in order to avoid modifying the ILI9341 crate. In what is probably best practice, the ILI9341 crate doesn't expose the raw write interface and forces any input to conform to embedded HAL standards. This frustrates the C programmer within me, which feels the right to manipulate memory as I see fit, damn the footguns. However, I capitulated and made a nifty iterator that parses the encoded image and spits out U16s in the right order, efficiently pushing the data in a continuous write to the TFT's framebuffer without overhead to re-point the display pointer.
