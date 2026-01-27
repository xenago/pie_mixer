# pie_mixer

Mix multiple stereo inputs to one output.

## Notes

* Basic digital mixer for Raspberry Pi with no options
* Currently designed for S/PDIF digital audio ports but can be made to work with any PipeWire source/sink

## Hardware

* Dedicated Raspberry Pi 4
* 1x [UGREEN USB Hub 3.0, 4 Ports USB A](https://amazon.ca/dp/B0CD1BHXPZ)
* 1x [Cubilux USB A to TOSLINK Optical Audio Adapter](https://amazon.ca/dp/B0D2L27B7B) (16/24-bit@44.1/48/96/192KHz)
* 2-5x [Cubilux USB A SPDIF Input Adapter](https://amazon.ca/dp/B0DFW6DLF2) (16-bit@44.1/48/96KHz)

## Software

1. Install Ubuntu 24.04 and set up system fresh

2. Install build and runtime dependencies:

       sudo apt-get install curl build-essential clang git libpipewire-0.3-dev pipewire pkg-config

3. Install Rust:

       curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
       . "$HOME/.cargo/env"

4. Download `pie_mixer`:

       cd ~
       git clone https://github.com/xenago/pie_mixer.git

5. Build:

       cd pie_mixer
       bash -c 'cargo build --release'

6. Configure PipeWire

   Set allowed output sample rates (increasing this will require substantially more CPU resources):

       pw-metadata -n settings 0 clock.allowed-rates "[ 44100 48000 96000 ]"

   Optionally, set a default value:

       pw-metadata -n settings 0 clock.rate 44100

   Optionally, force a specific value:

       pw-metadata -n settings 0 clock.force-rate 96000

   Optionally, reduce buffer (can be unstable below 1024):

       pw-metadata -n settings 0 clock.force-quantum 768

7. Run:

       ./target/release/pie_mixer

   e.g.

       user@rpi4:~/pie_mixer$ ./target/release/pie_mixer
       2026-01-27T08:36:37.452390Z  INFO pie_mixer: pie_mixer init...
       2026-01-27T08:36:37.466935Z  INFO pie_mixer: PipeWire nodes found: 12
       2026-01-27T08:36:37.467002Z  INFO pie_mixer: Matching inputs: 2
       2026-01-27T08:36:37.467022Z  INFO pie_mixer: Matching outputs: 1
       2026-01-27T08:36:37.467037Z  INFO pie_mixer: Configuring mixer...
       2026-01-27T08:36:37.467159Z  INFO pie_mixer: Mixer links established!
       2026-01-27T08:36:37.467193Z  INFO pie_mixer: Keep program active to maintain connections, or press Ctrl+C to stop the mixer...
       ^C

### Debugging

Example:

    user@rpi4:~/pie_mixer$ cargo build
    user@rpi4:~/pie_mixer$ RUST_LOG=DEBUG ./target/debug/pie_mixer
    2026-01-27T08:36:41.023956Z  INFO pie_mixer: pie_mixer init...
    2026-01-27T08:36:41.038364Z  INFO pie_mixer: PipeWire nodes found: 12
    2026-01-27T08:36:41.038775Z DEBUG pie_mixer: [ID:  29]  Description: Dummy-Driver                                [Type: Other/Virtual]  Ports: []
    2026-01-27T08:36:41.038901Z DEBUG pie_mixer: [ID:  30]  Description: Freewheel-Driver                            [Type: Other/Virtual]  Ports: []
    2026-01-27T08:36:41.038963Z DEBUG pie_mixer: [ID:  36]  Description: Built-in Audio Stereo                       [Type:  Audio Output]  Ports: [(80, "FL", "in"), (81, "FL", "out"), (82, "FR", "in"), (83, "FR", "out")]
    2026-01-27T08:36:41.039038Z DEBUG pie_mixer: [ID:  40]  Description: Cubilux SPDIF ReceiverSolid  Analog Stereo  [Type:   Audio Input]  Ports: [(88, "FL", "out"), (89, "FR", "out")]
    2026-01-27T08:36:41.039094Z DEBUG pie_mixer: [ID:  42]  Description: USB SPDIF Adapter Analog Stereo             [Type:  Audio Output]  Ports: [(84, "FL", "in"), (85, "FL", "out"), (86, "FR", "in"), (87, "FR", "out")]
    2026-01-27T08:36:41.039175Z DEBUG pie_mixer: [ID:  43]  Description: Midi-Bridge                                 [Type: Other/Virtual]  Ports: [(44, "Midi Through:(playback_0) Midi Through Port-0", "in"), (45, "Midi Through:(capture_0) Midi Through Port-0", "out")]
    2026-01-27T08:36:41.039267Z DEBUG pie_mixer: [ID:  66]  Description: bcm2835-isp (V4L2)                          [Type:   Video Input]  Ports: [(67, "capture_1", "out")]
    2026-01-27T08:36:41.039343Z DEBUG pie_mixer: [ID:  68]  Description: bcm2835-isp (V4L2)                          [Type:   Video Input]  Ports: [(69, "capture_1", "out")]
    2026-01-27T08:36:41.039406Z DEBUG pie_mixer: [ID:  70]  Description: bcm2835-isp (V4L2)                          [Type:   Video Input]  Ports: [(71, "capture_1", "out")]
    2026-01-27T08:36:41.039499Z DEBUG pie_mixer: [ID:  72]  Description: bcm2835-isp (V4L2)                          [Type:   Video Input]  Ports: [(73, "capture_1", "out")]
    2026-01-27T08:36:41.039585Z DEBUG pie_mixer: [ID:  74]  Description: Cubilux SPDIF ReceiverSolid  Analog Stereo  [Type:   Audio Input]  Ports: [(41, "FL", "out"), (39, "FR", "out")]
    2026-01-27T08:36:41.039659Z DEBUG pie_mixer: [ID:  75]  Description: Built-in Audio Digital Stereo (HDMI)        [Type:  Audio Output]  Ports: [(76, "FL", "in"), (77, "FL", "out"), (78, "FR", "in"), (79, "FR", "out")]
    2026-01-27T08:36:41.039819Z  INFO pie_mixer: Matching inputs: 2
    2026-01-27T08:36:41.039907Z DEBUG pie_mixer: [ID:  40] Cubilux SPDIF ReceiverSolid  Analog Stereo
    2026-01-27T08:36:41.039957Z DEBUG pie_mixer: [ID:  74] Cubilux SPDIF ReceiverSolid  Analog Stereo
    2026-01-27T08:36:41.040042Z  INFO pie_mixer: Matching outputs: 1
    2026-01-27T08:36:41.040105Z DEBUG pie_mixer: [ID:  42] USB SPDIF Adapter Analog Stereo
    2026-01-27T08:36:41.040162Z  INFO pie_mixer: Configuring mixer...
    2026-01-27T08:36:41.040210Z DEBUG pie_mixer: Mapping all matching inputs to output [ID: 42, USB SPDIF Adapter Analog Stereo]
    2026-01-27T08:36:41.040265Z DEBUG pie_mixer: Stereo linking: [ID: 40, Cubilux SPDIF ReceiverSolid  Analog Stereo]=>[ID: 42, USB SPDIF Adapter Analog Stereo]
    2026-01-27T08:36:41.040341Z DEBUG pie_mixer: Linking channel FL: [88]->[84]
    2026-01-27T08:36:41.040558Z DEBUG pie_mixer: Linking channel FR: [89]->[86]
    2026-01-27T08:36:41.040734Z DEBUG pie_mixer: Stereo linking: [ID: 74, Cubilux SPDIF ReceiverSolid  Analog Stereo]=>[ID: 42, USB SPDIF Adapter Analog Stereo]
    2026-01-27T08:36:41.040828Z DEBUG pie_mixer: Linking channel FL: [41]->[84]
    2026-01-27T08:36:41.040914Z DEBUG pie_mixer: Linking channel FR: [39]->[86]
    2026-01-27T08:36:41.041013Z  INFO pie_mixer: Mixer links established!
    2026-01-27T08:36:41.041062Z  INFO pie_mixer: Keep program active to maintain connections, or press Ctrl+C to stop the mixer...
