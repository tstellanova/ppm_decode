# ppm_decode
PPM decoding for no_std rust. 
This library decodes the commonly-used PPM format used in radio control 
and other embedded applications. 


## Example

Typically PPM pulse input might be received via an input pin interrupt.
The important thing is that you provide this parser with the time
of the start of a pulse. In PPM the only time difference that 
matters is the difference between consecutive pulses. 

See `PpmParser` documentation for example usage,
or refer to the 
[test_ppm_decode](https://github.com/tstellanova/test_ppm_decode)
project for an example using an stm32f4 microcontroller. 

## Status

- [x] Basic parsing of anonymous PPM  time events
- [x] Basic tests
- [x] Usage example
- [x] Test clock overflow
- [ ] Test unusual configuration limits
- [ ] Locks onto a consistent number of channels over time
- [x] Doc comments and example
- [ ] CI
