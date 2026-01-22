# confetti

a framework for building FRC robots in Rust

built on [guineawheek/wpihal-rs](https://github.com/guineawheek/wpihal-rs).

## goals

### WILL NOT abide by the WPILIB API

there are a lot of things they do that would be bizarre at best to do in Rust (from my understanding). i also don't like it that much anyway

### WILL go for feature parity

unfortunately, my team has limited time, people, and resources, so this is largely a personal project and there is very limited hardware available for me to test. implementing hardware is something that i will likely have to rely on others for

### WILL go for ease-of-use

i want this library to be approachable for beginner Rust programmers, so i will try hard to make things make sense.

# roadmap
- [ ] get robot loops to work
- [ ] implement revlib
- [ ] implement command-based style framework
- [ ] implement wpimath (sigh)
- [ ] get CLI in order
  - [ ] deploy
  - [ ] project init
