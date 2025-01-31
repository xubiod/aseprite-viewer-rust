<div style="text-align: center" align="center">
<h1>Another Aseprite Viewer</h1>
<h3>Written in Rust with the use of <a href="http://raylib.com/" rel="noreferrer noopener" target="_blank">Raylib</a></h3>
</div>

---

Similar concept to my other Aseprite viewer made completely in
[Godot and GDScript](https://github.com/xubiod/aseprite-file-viewer). There are
some differences, though.
- All cels are displayed a grid-like view
- Scroll wheel zooms
- Right mouse button pans the view
- Tags are read, but at the time of writing are not used
- Blend modes are read and written out in layer properties but don't affect the rendering
- Cels are not clipped to the sprite size
  - Reference layers are shown because of this, however they are not positioned properly

## Motivation

Main motivations behind this project have been:
- Learn Rust more
- Learn Raylib through the [raylib-rs crate](https://crates.io/crates/raylib)
  - By extension rgui

## License

This source code is licensed under [MIT](LICENSE).