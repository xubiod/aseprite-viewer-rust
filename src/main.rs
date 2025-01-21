mod ui;
mod ase;

use std::{fs::File, io};

use ase::aseprite;
use ui::ui_main;

fn main() -> io::Result<()> {
    ui_main::ui();
    Ok(())
    
    // open_test()
}

#[allow(dead_code)]
fn open_test() -> io::Result<()> {
    let fpath = "select.aseprite";
    let mut f_in = File::open(fpath)?;

    let data = aseprite::read(&mut f_in).unwrap();

    println!("{0}\nheader\n\t{1}b\n\tcanvas {2} by {3}\n\tgrid @ {4}, {5}; {6} by {7}\n\t{8}bpp, {9} colours",
        fpath, 
        data.header.fsize, 
        data.header.width, data.header.height, 
        data.header.grid_xpos, data.header.grid_ypos, data.header.grid_width, data.header.grid_height,
        data.header.colour_depth, data.header.colour_count
    );
    println!("frames");
    for f in data.frames {
        println!("\t{0}ms\n\t{1} chunks", f.frame_duration, f.chunk_count);
        for c in f.chunks {
            print!("\t\t{0}", c.name());
            match c {
                aseprite::Chunk::Unknown(rchunk) => {
                    println!("\t\ttype x{0:04x} {1}b", rchunk.chunk_type, rchunk.size)
                },
                aseprite::Chunk::Layer(lchunk) => {
                    println!("\t{0}\n\t\t\tblend {1} at {2} opacity\n\t\t\tchild lvl {3}", 
                        lchunk.name.as_str().unwrap(),
                        lchunk.blend_mode, lchunk.opacity,
                        lchunk.child_level
                    )
                },
                aseprite::Chunk::Cel(cchunk)  => {
                    println!("\t{4}\n\t\t\t@ on layer idx {7}; {0}, {1}; {2} by {3}\n\t\t\t{5}b\n\t\t\tlinked to {6}",
                        cchunk.x_pos, cchunk.y_pos, cchunk.width.unwrap_or(0), cchunk.height.unwrap_or(0),
                        cchunk.cel_type,
                        cchunk.raw_data.unwrap_or(cchunk.compressed_data.unwrap_or(vec![0].into()).into()).len(),
                        cchunk.linked_to.unwrap_or(0xFFFF),
                        cchunk.layer_index
                    )
                },
                aseprite::Chunk::Tag(tchunk)  => {
                    println!("\tcount {0}", tchunk.tag_count);
                    for t in tchunk.tags {
                        println!("\t\t\t\t{0}\n\t\t\t\t{1} -> {2}, going {3} {4} times",
                            t.name.as_str().unwrap(),
                            t.from, t.to, t.direction, t.repeat_count
                        )
                    }
                },
            }
        }
    }

    Ok(())
}