use rug::*;

fn main() {
    let mut pb = path::PathBuilder::new();
    pb.move_to([1.0, 1.0]);
    pb.line_to([9.0, 1.0]);
    pb.line_to([5.0, 4.0]);
    pb.close_path();

    let path_buf = pb.build();
    let path = path_buf.path();

    println!("{:?}", path.verbs());
    println!("{:?}", path.points());
    println!("{:?}", path.aabb());

    for e in path.iter() {
        println!("{:?}", e);
    }


    let cmds = cmd::CmdBuf::new(|cb| {
        let path = cb.build_path(|pb| {
            pb.move_to([1.0, 1.0]);
            pb.line_to([9.0, 1.0]);
            pb.line_to([5.0, 4.0]);
            pb.close_path();
        });

        cb.push(cmd::Cmd::FillPathSolid { path, color: 42 });
    });
    println!("{:?}", cmds.num_cmds());
}

