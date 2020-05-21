# ppm_decode
PPM decoding for embedded hal , no_std rust


## Example

Typically PPM input might be received via an input pin interrupt.
The important thing is that you provide this parser with the time
of the start of a pulse. In PPM the only time difference that 
matters is the difference between consecutive pulses. 

```rust
    let mut parser = PpmParser::new();   
    let mut cur_time: PpmTime = 100;
 
    loop {
        parser.handle_pulse_start(cur_time);    
        if let Some(frame) = parser.next_frame() {
            //TODO process the frame
        }

        cur_time += 100; //TODO get from clock, interrupt, or whatever
    }   
``` 

## Status

- [x] Basic parsing of anonymous PPM  time events
- [x] Basic tests
- [x] Usage example
- [ ] Test clock overflow
- [ ] Test unusual configuration limits
- [ ] Locks onto a consistent number of channels over time
- [ ] Doc comments
- [ ] CI
